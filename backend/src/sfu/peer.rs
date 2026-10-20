use axum::extract::ws::Message;
use futures_util::{FutureExt, select};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;
use webrtc::data_channel::{DataChannel, DataChannelEvent};
use webrtc::media_stream::track_remote::{TrackRemote, TrackRemoteEvent};
use webrtc::peer_connection::{PeerConnectionEventHandler, RTCIceConnectionState, RTCPeerConnectionIceErrorEvent, RTCPeerConnectionIceEvent};

pub struct PcHandler {
    #[allow(dead_code)]
    pub con_id: Uuid,
    pub conn: mpsc::Sender<Message>,
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
        if self.conn.send(Message::Text(msg.into())).await.is_err() {
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
        info!("data channel");
        tokio::spawn(Box::pin(async move {
            loop {
                select! {
                    event = dc.poll().fuse() => {
                        match event {
                            Some(DataChannelEvent::OnBufferedAmountHigh) => {
                                info!("data channel: buffered amount high");
                            }
                            Some(DataChannelEvent::OnBufferedAmountLow) => {
                                info!("data channel: buffered amount low");
                            }
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
        let con_id = self.con_id;
        tokio::spawn(async move {
            let mut count: u64 = 0;
            while let Some(evt) = track.poll().await {
                match evt {
                    TrackRemoteEvent::OnRtpPacket(_pkt) => {
                        // TODO: forward ke subscriber (SFU)
                        count += 1;
                        if count.is_multiple_of(300) {
                            info!("conn {con_id}: {count} rtp packets");
                        }
                    }
                    TrackRemoteEvent::OnEnded => {
                        info!("conn {con_id}: track ended");
                        break;
                    }
                    _ => {}
                }
            }
        });
    }
}
