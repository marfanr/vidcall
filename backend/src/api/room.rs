use axum::{
    Json, Router,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode, header::USER_AGENT},
    response::Response,
    routing::{get, post},
};
use futures_util::{FutureExt, SinkExt, StreamExt, select};
use sonic_rs::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc};
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;
use validator::Validate;
use webrtc::{
    data_channel::{DataChannel, DataChannelEvent},
    media_stream::track_remote::{TrackRemote, TrackRemoteEvent},
    peer_connection::{
        MediaEngine, PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler,
        RTCConfigurationBuilder, RTCIceConnectionState, RTCIceServer,
        RTCPeerConnectionIceErrorEvent, RTCPeerConnectionIceEvent, RTCSessionDescription,
        SettingEngineBuilder, crypto::providers::RingProvider,
    },
    runtime::TokioRuntime,
};

use crate::{
    api::{
        ApiError::{self, Rejection},
        FastJson, ValidateJson,
        server::ServerState,
    },
    models::room::RoomId,
};

#[derive(Deserialize, Validate)]
struct CreateRoomReq {
    #[validate(length(min = 1, max = 64))]
    name: String,
}

#[derive(Serialize)]
struct CreateRoomRes {
    token: Option<&'static str>,
}

#[derive(Default)]
pub struct V1 {}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsMsg<'a> {
    Join {
        room: &'a str,
    },
    Offer {
        sdp: String,
    },
    Answer {
        _sdp: String,
    },
    Candidate {
        candidate: String,
        mid: Option<String>,
        line_index: Option<u16>,
        username_fragment: Option<String>,
    },
    Leave,
}

struct PcHandler {
    #[allow(dead_code)]
    con_id: Uuid,
    conn: mpsc::Sender<Message>,
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

    async fn on_ice_connection_state_change(&self, state: RTCIceConnectionState) {
        info!("on ice state change {}", state.to_owned());
    }

    async fn on_ice_candidate_error(&self, event: RTCPeerConnectionIceErrorEvent) {
        warn!("ice candidate error: {:?}", event);
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
                        if count % 300 == 0 {
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
}

impl<'a> WsMsg<'a> {
    async fn handle_join(room_id: &'a str, con_id: Uuid, state: &ServerState) {
        info!("join room={} from {}", room_id, con_id);

        let _room = state
            .rooms
            .get(room_id)
            .map(|r| Arc::clone(r.value()))
            .ok_or_else(|| {
                return
            }).unwrap();

        // r.connection.push(Arc::from(con_id.to_string()));
        info!("joined room {}", room_id);
    }

    async fn handle_offer(
        sdp: &str,
        con_id: Uuid,
        state: &ServerState,
        self_sink: mpsc::Sender<Message>,
    ) -> Result<(), String> {
        let config = RTCConfigurationBuilder::default()
            .with_ice_servers(vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }])
            .build();

        let se = SettingEngineBuilder::default()
            .with_crypto_provider(Arc::new(RingProvider::new()))
            .with_allow_insecure_verification_algorithm(true)
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
                con_id,
                conn: self_sink.clone(),
            }))
            .with_udp_addrs(vec!["0.0.0.0:0"])
            .with_setting_engine(se)
            .build()
            .await
            .map_err(|e| format!("build pc: {e}"))?;

        let offer = RTCSessionDescription::offer(sdp.to_owned())
            .map_err(|e| format!("parse offer: {e}"))?;
        pc.set_remote_description(offer)
            .await
            .map_err(|e| format!("remote desc: {e}"))?;

        let answer = pc
            .create_answer(None)
            .await
            .map_err(|e| format!("create answer: {e}"))?;
        pc.set_local_description(answer.clone())
            .await
            .map_err(|e| format!("local desc: {e}"))?;

        for l in answer.sdp.lines().filter(|l| {
            l.starts_with("m=") || l.starts_with("a=ice-ufrag") || l.starts_with("a=rtpmap")
        }) {
            info!("answer: {}", l.trim());
        }

        self_sink
            .send(Message::text(
                sonic_rs::json!({ "type": "answer", "sdp": answer.sdp }).to_string(),
            ))
            .await
            .map_err(|e| format!("send answer: {e}"))?;

        state.pcs.write().await.insert(con_id, Arc::from(pc));
        Ok(())
    }
}

// Api Version 1
impl V1 {
    pub fn routes() -> Router<ServerState> {
        Router::new()
            .route("/", post(Self::create_room))
            // .route("/", get(Self::get_room))
            .route("/ws", get(Self::ws))
    }

    async fn create_room(
        State(state): State<ServerState>,
        ValidateJson(r): ValidateJson<CreateRoomReq>,
    ) -> Result<(StatusCode, FastJson<CreateRoomRes>), ApiError> {
        {
            // let id = RoomId::from_str(r.name.into()).unwrap();
            // rooms.push(crate::models::room::Room {
            //     id: Arc::from(Uuid::now_v7().to_string()),
            //     name: Arc::from(r.name),
            //     connection: Vec::new(),
            // });
            // info!("rooms : {:?}", &rooms);
        }

        Ok((StatusCode::OK, FastJson(CreateRoomRes { token: None })))
    }

    // async fn get_room(State(state): State<ServerState>) -> Result<Json<Vec<String>>, ApiError> {
    //     // let rooms = state.rooms.read().await;
    //     // let room_ids: Vec<String> = rooms.iter().map(|r| r.id.to_string()).collect();
    //     Ok(Json())
    // }

    async fn ws(
        ws: WebSocketUpgrade,
        headers: HeaderMap,
        State(state): State<ServerState>,
    ) -> Response {
        let user_agent = headers
            .get(USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("Unknown")
            .to_owned();

        let con_id = Uuid::now_v7();
        ws.on_upgrade(move |socket| Self::handle_socket(socket, user_agent, state, con_id))
    }

    async fn handle_socket(
        socket: WebSocket,
        _user_agent: String,
        state: ServerState,
        con_id: Uuid,
    ) {
        let (mut sink, mut stream) = socket.split();
        let (tx, mut rx) = mpsc::channel::<Message>(64);
        state.connections.insert(con_id, tx.clone());

        loop {
            tokio::select! {
                incoming = stream.next() => {
                    match incoming {
                        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                        Some(Ok(Message::Text(msg))) => {
                            match sonic_rs::from_slice::<WsMsg<'_>>(msg.as_bytes()) {
                                Ok(WsMsg::Join { room }) => {
                                    WsMsg::handle_join(room, con_id, &state).await;
                                }
                                Ok(WsMsg::Offer { sdp }) => {
                                    if let Err(e) = WsMsg::handle_offer(&sdp, con_id, &state, tx.clone()).await {
                                        warn!("handle_offer: {e}");
                                    }
                                }
                                Ok(WsMsg::Candidate { candidate, line_index, mid, username_fragment }) => {
                                    info!("received candidate: {}", candidate);
                                    let pc = state.pcs.read().await.get(&con_id).cloned();
                                    match pc {
                                        Some(pc) => {
                                            if let Err(e) = pc
                                                .add_ice_candidate(webrtc::peer_connection::RTCIceCandidateInit {
                                                    candidate,
                                                    sdp_mid: mid,
                                                    sdp_mline_index: line_index,
                                                    username_fragment,
                                                    url: None,
                                                })
                                                .await
                                            {
                                                warn!("add_ice_candidate: {e}");
                                            }
                                        }
                                        None => warn!("candidate before pc exists"),
                                    }
                                }
                                Ok(WsMsg::Leave) => break,
                                Ok(_) => {}
                                Err(e) => warn!("bad ws msg: {e}"),
                            }
                        }
                        Some(Ok(_)) => {}
                    }
                }

                outgoing = rx.recv() => {
                    match outgoing {
                        Some(msg) => {
                            if sink.send(msg).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            }
        }

        state.connections.remove(&con_id);
        state.pcs.write().await.remove(&con_id);

        info!("ws disconnected");
    }
}
