use crate::sfu::data_channel::handle_data_channel;
use crate::sfu::room::RoomSfu;
use axum::extract::ws::Message;
use futures_util::{FutureExt, select};
use rtc::ice::mdns::MulticastDnsMode;
use rtc::ice::network_type::NetworkType;
use std::sync::{Arc, OnceLock};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::{Instant, timeout};
use tracing::{info, warn};
use uuid::Uuid;
use webrtc::data_channel::{DataChannel, DataChannelEvent};
use webrtc::media_stream::track_remote::{TrackRemote, TrackRemoteEvent};
use webrtc::peer_connection::crypto::providers::RingProvider;
use webrtc::peer_connection::{
    MediaEngine, PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler,
    RTCConfigurationBuilder, RTCIceConnectionState, RTCIceServer, RTCPeerConnectionIceErrorEvent,
    RTCPeerConnectionIceEvent, SettingEngineBuilder,
};
use webrtc::runtime::TokioRuntime;

static PROVIDER: OnceLock<Arc<RingProvider>> = OnceLock::new();

pub async fn create_peer(
    conn_id: Uuid,
    self_sink: mpsc::Sender<Message>,
    room: Arc<RoomSfu>,
) -> Result<Arc<dyn PeerConnection>, String> {
    let provider = PROVIDER
        .get()
        .map(|provider| Arc::clone(provider))
        .unwrap_or_else(|| {
            let p = Arc::new(RingProvider::new());
            _ = PROVIDER.set(Arc::clone(&p));
            p.clone()
        });

    let config = RTCConfigurationBuilder::default()
        .with_ice_servers(vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }])
        .build();

    let se = SettingEngineBuilder::default()
        .with_crypto_provider(provider.clone())
        .with_allow_insecure_verification_algorithm(false)
        .with_multicast_dns_mode(MulticastDnsMode::Disabled)
        .with_network_types(vec![NetworkType::Udp4])
        .build();

    let mut media_engine = MediaEngine::default();
    media_engine
        .register_default_codecs()
        .map_err(|e| format!("register codecs: {e}"))?;

    let pc = PeerConnectionBuilder::new()
        .with_configuration(config)
        .with_media_engine(media_engine)
        .with_runtime(Arc::new(TokioRuntime))
        .with_handler(Arc::new(PcHandler {
            conn_id: conn_id,
            ws_conn: self_sink.clone(),
            room: room.clone(),
        }))
        .with_udp_addrs(vec!["0.0.0.0:0"])
        .with_setting_engine(se)
        .build()
        .await
        .map_err(|e| format!("build pc: {e}"))?;

    Ok(Arc::new(pc))
}

pub struct PcHandler {
    pub conn_id: Uuid,
    pub ws_conn: mpsc::Sender<Message>,
    pub room: Arc<RoomSfu>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for PcHandler {
    async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
        let init = match event.candidate.to_json() {
            Ok(t) => t,
            Err(e) => {
                warn!("candidate to_json: {e}");
                return;
            }
        };
        info!("ice detected : {}", init.candidate);
        let msg = sonic_rs::json!({
            "type": "ice",
            "candidate": &init.candidate,
            "sdpMid": &init.sdp_mid,
            "sdpMLineIndex": &init.sdp_mline_index,
            "usernameFragment": &init.username_fragment,
        })
        .to_string();
        if self.ws_conn.send(Message::Text(msg.into())).await.is_err() {
            warn!("ws closed, drop ice candidate");
        }
    }

    async fn on_ice_candidate_error(&self, event: RTCPeerConnectionIceErrorEvent) {
        warn!("ice candidate error: {:?}", event);
    }

    async fn on_ice_connection_state_change(&self, state: RTCIceConnectionState) {
        info!("on ice state change {}", state.clone());
    }

    async fn on_data_channel(&self, dc: Arc<dyn DataChannel>) {
        let conn_id = self.conn_id;
        let room = Arc::clone(&self.room);

        tokio::spawn(Box::pin(async move {
            let label = Arc::from(dc.label().await.unwrap_or_default());
            loop {
                select! {
                    event = dc.poll().fuse() => {
                        match event {
                            Some(DataChannelEvent::OnMessage(msg)) =>
                                handle_data_channel(
                                    conn_id,
                                    msg,
                                    Arc::clone(&label),
                                    room.clone()
                                ).await,
                            Some(DataChannelEvent::OnClose) | None => break,
                            _ => {}
                        }
                    }
                }
            }
        }));
    }

    async fn on_track(&self, track: Arc<dyn TrackRemote>) {
        info!("found track");
        let con_id = self.conn_id.clone();
        let room = self.room.clone();

        let Some(ssrc) = track.ssrcs().await.first().copied() else {
            warn!("conn {con_id}: track without ssrc");
            return;
        };
        let kind = track.kind().await;
        let Some(codec) = track.codec(ssrc).await else {
            warn!("conn {con_id}: no codec for ssrc {ssrc}");
            return;
        };

        let published = room.publish_track(con_id, kind, codec).await;
        let is_video = kind == rtc::rtp_transceiver::rtp_sender::RtpCodecKind::Video;

        tokio::spawn(async move {
            let mut count: u64 = 0;
            let idle_timeout = Duration::from_millis(1500);
            let pli_interval = Duration::from_millis(250);
            let pli_window = Duration::from_secs(3);

            let mut pli_until = Instant::now() + pli_window;
            let mut last_pli: Option<Instant> = None;
            let mut last_rtp = Instant::now();

            loop {
                match timeout(idle_timeout, track.poll()).await {
                    Ok(Some(TrackRemoteEvent::OnRtpPacket(pkt))) => {
                        last_rtp = Instant::now();
                        if room.set_track_active(&published, true).await {
                            pli_until = Instant::now() + pli_window;
                            info!("conn {con_id}: track resumed");
                        }
                        published.forward(&pkt).await;
                        count += 1;
                        if count.is_multiple_of(300) {
                            info!("conn {con_id}: {count} rtp packets");
                        }
                    }
                    Ok(Some(TrackRemoteEvent::OnEnded)) => {
                        info!("conn {con_id}: track ended explicitly");
                        break;
                    }
                    Ok(None) => {
                        info!("conn {con_id}: stream channel closed");
                        break;
                    }
                    Ok(Some(_)) => {}
                    Err(e) => {}
                }

                if last_rtp.elapsed() > idle_timeout
                    && room.set_track_active(&published, false).await
                {
                    info!("conn {con_id}: track idle, paused");
                }

                if is_video
                    && published.is_active()
                    && Instant::now() < pli_until
                    && last_pli.map_or(true, |t| t.elapsed() >= pli_interval)
                {
                    let pli = Box::new(
                        rtc::rtcp::payload_feedbacks::picture_loss_indication::PictureLossIndication {
                            sender_ssrc: 0,
                            media_ssrc: ssrc,
                        },
                    );
                    if let Err(e) = track.write_rtcp(vec![pli]).await {
                        warn!("conn {con_id}: failed to send PLI: {e}");
                    }

                    last_pli = Some(Instant::now());
                }
            }

            room.unpublish_track(&published).await;
            info!("conn {con_id}: track successfully unpublished and cleaned up");
        });
    }
}
