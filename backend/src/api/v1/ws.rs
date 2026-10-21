use crate::api::WsError;
use crate::api::server::ServerState;
use crate::models;
use crate::sfu::peer::PcHandler;
use axum::Router;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use axum::routing::get;
use futures_util::{TryFutureExt, sink::SinkExt, stream::StreamExt};
use http::header::{ORIGIN, USER_AGENT};
use http::{HeaderMap, StatusCode};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};
use uuid::Uuid;
use webrtc::peer_connection::crypto::providers::RingProvider;
use webrtc::peer_connection::{
    MediaEngine, PeerConnection, PeerConnectionBuilder, RTCConfigurationBuilder, RTCIceServer,
    RTCSessionDescription, SettingEngineBuilder,
};
use webrtc::runtime::TokioRuntime;

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsMsg {
    Join {
        room: Uuid,
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

impl WsMsg {
    /// handle joining a room
    async fn handle_join(
        room_id: Uuid,
        con_id: Uuid,
        state: &ServerState,
        self_sink: mpsc::Sender<Message>,
    ) -> Result<(), WsError> {
        info!("join room={} from {}", room_id, con_id);
        // created room object on memory only if owners already joined
        let db = state
            .pool
            .get()
            .map_err(|_| WsError(Arc::from("sasa")))
            .await;

        // let room = models::room::Room::get(&db, room_id)
        //     .map_err(|_err| WsError(Arc::from("room not found"))?
        //     .await?;
        //
        // println!("join room={:?}", room);

        // r.connection.push(Arc::from(con_id.to_string()));

        // TODO: handle accept deny
        self_sink
            .send(Message::text(
                sonic_rs::json!({ "type": "join", "accept": true }).to_string(),
            ))
            .await
            .map_err(|e| format!("send answer: {e}"))?;

        Ok(())
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

        // state.pcs.insert(con_id, Arc::from(pc));
        Ok(())
    }
}

pub fn routes() -> Router<ServerState> {
    Router::new().route("/", get(self::ws))
}

async fn ws(
    State(s): State<ServerState>,
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<ServerState>,
) -> Result<Response, StatusCode> {
    let origin = headers
        .get(ORIGIN)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::FORBIDDEN)?;

    if !s.allowed_origins.iter().any(|o| o == origin) {
        return Err(StatusCode::FORBIDDEN);
    }

    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("Unknown")
        .to_owned();

    let con_id = Uuid::now_v7();
    Ok(ws.on_upgrade(move |socket| self::handle_socket(socket, user_agent, state, con_id)))
}

async fn handle_socket(socket: WebSocket, _user_agent: String, state: ServerState, con_id: Uuid) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Message>(64);
    state.wsconn.insert(con_id, tx.clone());

    loop {
        tokio::select! {
            incoming = stream.next() => {
                match incoming {
                    Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                    Some(Ok(Message::Text(msg))) => {
                        match sonic_rs::from_slice::<WsMsg>(msg.as_bytes()) {
                            Ok(WsMsg::Join { room }) => {
                                if let Err(e) = WsMsg::handle_join(room, con_id, &state, tx.clone()).await {
                                    warn!("handle_offer: {e}");
                                }
                            }
                            Ok(WsMsg::Offer { sdp }) => {
                                if let Err(e) = WsMsg::handle_offer(&sdp, con_id, &state, tx.clone()).await {
                                    warn!("handle_offer: {e}");
                                }
                            }
                            Ok(WsMsg::Candidate { candidate, line_index, mid, username_fragment }) => {
                                info!("received candidate: {}", candidate);
                                // let pc = state.pcs.get(&con_id).map(|p| Arc::clone(p.value()));
                                // match pc {
                                //     Some(pc) => {
                                //         if let Err(e) = pc
                                //             .add_ice_candidate(webrtc::peer_connection::RTCIceCandidateInit {
                                //                 candidate,
                                //                 sdp_mid: mid,
                                //                 sdp_mline_index: line_index,
                                //                 username_fragment,
                                //                 url: None,
                                //             })
                                //             .await
                                //         {
                                //             warn!("add_ice_candidate: {e}");
                                //         }
                                //     }
                                //     None => warn!("candidate before pc exists"),
                                // }
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

    state.wsconn.remove(&con_id);
    // state.pcs.remove(&con_id);

    info!("ws disconnected");
}
