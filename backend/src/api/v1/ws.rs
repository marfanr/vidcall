use crate::api::WsError;
use crate::api::server::ServerState;
use crate::models;
use crate::sfu::peer::create_peer;
use crate::sfu::room::{RoomPeer, RoomSfu};
use crate::utils::{AccessClaims, verify_jwt};
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
use webrtc::peer_connection::{RTCIceCandidateInit, RTCSessionDescription};

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsMsg {
    Join {
        room: Uuid,
        token: Option<Arc<str>>,
        /// username only used if token is empty
        username: Option<Arc<str>>,
    },
    AnswerRequest {
        conn_id: Uuid,
        room_id: Uuid,
    },
    Offer {
        sdp: String,
    },
    Answer {
        sdp: String,
    },
    Candidate {
        candidate: String,
        mid: Option<String>,
        line_index: Option<u16>,
        username_fragment: Option<String>,
    },
    Leave,
}

async fn answer_join(
    sink: &mpsc::Sender<Message>,
    accept: bool,
    conn_id: Uuid,
) -> Result<(), WsError> {
    sink.send(Message::text(
        sonic_rs::json!({ "type": "join", "accept": accept, "conn_id": conn_id}).to_string(),
    ))
    .map_err(|e| WsError::from(e))
    .await
}

async fn sync_room_member(
    sink: &mpsc::Sender<Message>,
    r: &RoomSfu,
    conn_id: &Uuid,
) -> Result<(), WsError> {
    let members: Vec<_> = r
        .peers
        .iter()
        .filter(|p| p.key() != conn_id)
        .map(|p| {
            sonic_rs::json!({
                "conn_id": p.key(),
                "username": &*p.value().username,
            })
        })
        .collect();

    sink.send(Message::text(
        sonic_rs::json!({ "type": "sync", "members": members}).to_string(),
    ))
    .map_err(|e| WsError::from(e))
    .await
}

impl WsMsg {
    /// handle joining a room
    async fn handle_join(
        room_id: Uuid,
        conn_id: Uuid,
        token: Option<Arc<str>>,
        username: Option<Arc<str>>,
        state: &ServerState,
        self_sink: mpsc::Sender<Message>,
    ) -> Result<(), WsError> {
        let user_id = token
            .as_deref()
            .map(verify_jwt::<AccessClaims>)
            .transpose()
            .map_err(|e| WsError(Arc::from(e.to_string())))?
            .map(|c| c.sub.parse::<Uuid>())
            .transpose()
            .map_err(|e| WsError(Arc::from(e.to_string())))?;

        info!("join room={} from {} ", room_id, conn_id);

        /// created room object on memory only if owners already joined
        let db = state.pool.get().map_err(|e| WsError::from(e)).await?;

        /// gets the SFU Room exist, or create it from db if available
        let existing = state.rooms.get(&room_id).map(|r| Arc::clone(r.value()));
        let room_arc = match existing {
            Some(room) => room,
            None => {
                let model = models::room::Room::get(&db, room_id).await?;
                let fresh = Arc::new(RoomSfu::from(&model));

                if user_id != Some(fresh.owner_id) {
                    return Err(WsError(Arc::from("room not started")));
                }

                let inserted = {
                    let entry = state.rooms.entry(room_id).or_insert(fresh);
                    Arc::clone(entry.value())
                };
                inserted
            }
        };
        let room: &RoomSfu = room_arc.as_ref();

        let username = username.unwrap_or(Arc::from("Anonymous"));
        let username = match user_id {
            Some(id) => models::user::User::get_by_id(&db, id)
                .await
                .map(|u| u.username)
                .unwrap_or_else(|e| {
                    tracing::warn!(%id, error = %e, "failed to load user, using fallback name");
                    username
                }),
            None => username,
        };

        let peer = create_peer(conn_id, room_arc.clone(), Some(self_sink.clone()))
            .await
            .map_err(|e| WsError(Arc::from(e.to_string())))?;

        let room_peer = RoomPeer::new(user_id, Arc::clone(&username), peer, Some(self_sink.clone()))
            .await
            .map_err(|e| WsError(Arc::from(e.to_string())))?;
        room.add_peer(conn_id, room_peer);
        state.conn_to_rooms.insert(conn_id, Arc::clone(&room_arc));

        /// handle joining room request
        /// if owner directy enter the room
        if user_id == Some(room.owner_id) {
            answer_join(&self_sink, true, conn_id).await?;
            return Ok(());
        }

        /// if not owner send join requets to the owner of room
        /// TODO!: check on options first, is the room need request to join or can join directly
        room.send_to_owner(
            conn_id,
            Message::text(
                sonic_rs::json!({
                "type": "request", "from": {"conn_id": conn_id, "username": username}})
                .to_string(),
            ),
        );

        Ok(())
    }

    async fn handle_offer(
        sdp: &str,
        conn_id: Uuid,
        state: &ServerState,
        self_sink: mpsc::Sender<Message>,
    ) -> Result<(), String> {
        let room = state
            .conn_to_rooms
            .get(&conn_id)
            .ok_or_else(|| "room not found for conn".to_string())?;
        let peers = room
            .peers
            .get(&conn_id)
            .ok_or_else(|| "peer not found in room".to_string())?;
        let pc = Arc::clone(&peers.pc);

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

        self_sink
            .send(Message::text(
                sonic_rs::json!({ "type": "answer", "sdp": answer.sdp }).to_string(),
            ))
            .await
            .map_err(|e| format!("send answer: {e}"))?;

        // After initial offer/answer negotiation, attach any existing tracks from other peers
        room.attach_existing_tracks(conn_id, &peers).await;

        Ok(())
    }

    async fn handle_answer(sdp: &str, conn_id: Uuid, state: &ServerState) -> Result<(), String> {
        let room = state
            .conn_to_rooms
            .get(&conn_id)
            .ok_or_else(|| "room not found for conn".to_string())?;
        let peers = room
            .peers
            .get(&conn_id)
            .ok_or_else(|| "peer not found in room".to_string())?;
        let pc = Arc::clone(&peers.pc);

        let answer = RTCSessionDescription::answer(sdp.to_owned())
            .map_err(|e| format!("parse answer: {e}"))?;
        pc.set_remote_description(answer)
            .await
            .map_err(|e| format!("remote answer desc: {e}"))?;

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

    let conn_id = Uuid::now_v7();
    Ok(ws.on_upgrade(move |socket| self::handle_socket(socket, user_agent, state, conn_id)))
}

async fn handle_socket(socket: WebSocket, _user_agent: String, state: ServerState, conn_id: Uuid) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Message>(64);
    state.ws_conn.insert(conn_id, tx.clone());

    loop {
        tokio::select! {
            incoming = stream.next() => {
                match incoming {
                    Some(Ok(Message::Close(_)) | Err(_)) | None => break,
                    Some(Ok(Message::Text(msg))) => {
                        match sonic_rs::from_slice::<WsMsg>(msg.as_bytes()) {
                            Ok(WsMsg::Join { room, token, username }) => {
                                if let Err(e) = WsMsg::handle_join(
                                    room,
                                    conn_id,token, username, &state, tx.clone()).await {
                                    warn!("handle_join: {e}");
                                }
                            }
                            /// only forwarding the message from client
                            Ok(WsMsg::AnswerRequest {conn_id, room_id}) => {
                                if let Some(c) = state.ws_conn.get(&conn_id) {
                                    /// allow entry into the room
                                    answer_join(c.value(), true, conn_id).await.map_err(
                                        |e| warn!("answer_join: {e}")
                                    ).unwrap();

                                    /// TODO: broadcast to all user on rooms that someone has joined this room
                                    if let Some(room) = state.rooms.get(&room_id) {
                                        sync_room_member(c.value(), &room, &conn_id).await.map_err(
                                            |e| warn!("sync room member: {e}")
                                        ).unwrap();

                                        let username = room.peers.get(&conn_id)
                                            .map(|p| p.username.clone())
                                            .unwrap_or(Arc::from("Anonymous"));

                                        room.broadcast(conn_id, Message::text(
                                            sonic_rs::json!({
                                            "type": "welcome", "from": {"conn_id": conn_id, "username": username}})
                                            .to_string(),
                                        ));
                                    };
                                };
                            }
                            Ok(WsMsg::Offer { sdp }) => {
                                if let Err(e) = WsMsg::handle_offer(&sdp, conn_id, &state, tx.clone()).await {
                                    warn!("handle_offer: {e}");
                                }
                            }
                            Ok(WsMsg::Answer { sdp }) => {
                                if let Err(e) = WsMsg::handle_answer(&sdp, conn_id, &state).await {
                                    warn!("handle_answer: {e}");
                                }
                            }
                            Ok(WsMsg::Candidate { candidate, line_index, mid, username_fragment }) => {
                                info!("received candidate: {}", candidate);
                                /// clone the peer connection out of the map before awaiting,
                                /// holding a dashmap guard across an await point can deadlock
                                let pc = {
                                    let room = state.conn_to_rooms.get(&conn_id);
                                    room.and_then(|room| {
                                        room.peers
                                            .get(&conn_id)
                                            .map(|peer| Arc::clone(&peer.value().pc))
                                    })
                                };

                                match pc {
                                    Some(pc) => {
                                        if let Err(e) = pc
                                            .add_ice_candidate(RTCIceCandidateInit {
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

    // clear connection
    state.ws_conn.remove(&conn_id);
    if let Some(room) = state.conn_to_rooms.get(&conn_id) {
        println!("delete {} from room", &conn_id);
        let username = room
            .peers
            .get(&conn_id)
            .map(|p| p.username.clone())
            .unwrap_or(Arc::from("Anonymous"));

        room.broadcast(
            conn_id,
            Message::text(
                sonic_rs::json!({
                    "type": "disconnect", "conn_id": conn_id, "username": username
                })
                .to_string(),
            ),
        );
        room.drop_peer(conn_id).await;
    }

    // TODO! broadcast exit to inform some user exit from the room
    // TODO! delete room if no member

    info!("ws disconnected");
}
