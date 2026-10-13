use std::sync::Arc;
use axum::{
    Json,
    extract::{
        State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    http::{HeaderMap, StatusCode, header::USER_AGENT},
    response::Response,
};
use sonic_rs::{Deserialize, Serialize};
use tracing::{info, warn};
use uuid::Uuid;
use validator::Validate;

use crate::api::{ApiError::{self, Rejection}, FastJson, ValidateJson, server::ServerState};

#[derive(Deserialize, Validate)]
pub struct CreateRoomReq {
    #[validate(length(min = 1, max = 64))]
    name: String,
}

#[derive(Serialize)]
pub struct CreateRoomRes {
    token: Option<&'static str>,
}

#[derive(Default)]
pub struct V1 {}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMsg<'a> {
    Join { room: &'a str },
    Offer { sdp: String },
    Answer { sdp: String },
    Candidate { candidate: &'a str },
    Leave,
}

impl WsMsg<'_> {}

impl V1 {
    pub async fn create_room(
        State(state): State<ServerState>,
        ValidateJson(r): ValidateJson<CreateRoomReq>,
    ) -> Result<(StatusCode, FastJson<CreateRoomRes>), ApiError> {
        {
            let mut rooms = state.rooms.write().await;
            if rooms.iter().any(
                |room| room.id.as_ref() == r.name
            ) {
                return Err(Rejection(StatusCode::OK, "name already exist".to_string()));
            }
            rooms.push(crate::models::room::Room {
                id: Arc::from(Uuid::now_v7().to_string()),
                name: Arc::from(r.name),
                connection: Vec::new(),
            });
            info!("rooms : {:?}", &rooms);
        }

        Ok((StatusCode::OK, FastJson(CreateRoomRes { token: None })))
    }

    pub async fn get_room(State(state): State<ServerState>) -> Result<Json<Vec<String>>, ApiError> {
        let rooms = state.rooms.read().await;
        let room_ids: Vec<String> = rooms.iter().map(|r| r.id.to_string()).collect();
        Ok(Json(room_ids))
    }

    pub async fn ws(
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

    async fn handle_socket(mut socket: WebSocket, user_agent: String, state: ServerState, con_id: Uuid) {
        while let Some(msg) = socket.recv().await {
            match msg {
                Ok(Message::Text(t)) => match sonic_rs::from_slice::<WsMsg<'_>>(t.as_bytes()) {
                    Ok(WsMsg::Join { room}) => {
                        info!("join room={} from {}", room, con_id);
                        let rooms = state.rooms.read().await;

                        let exists = rooms.iter().any(|r| r.id.as_ref() == room);
                        if !exists {
                            continue;
                        }

                        info!("joined room {}", room);
                    }
                    Ok(WsMsg::Offer { sdp }) => {
                        info!("received offer: {} bytes", sdp.len());
                    }
                    Ok(WsMsg::Candidate { candidate }) => {
                        info!("received candidate: {} bytes", candidate.len());
                    }
                    Ok(WsMsg::Leave) => break,
                    Ok(_) => {}
                    Err(e) => { warn!("err : {}", e)}
                },
                Ok(Message::Close(_)) | Err(_) => break,
                Ok(_) => {}
            }
        }

        info!("ws disconnected: {}", user_agent);
    }
}
