use axum::{
    Router,
    extract::ws::{Message}
};

use dashmap::DashMap;
use deadpool_postgres::Pool;
use tokio::{net::TcpListener, sync::mpsc};
use tracing::info;

use crate::{api::{room, user}, models::room::{Room, RoomId}};

#[derive(Default)]
struct ApiV1 {}

impl ApiV1 {
    fn routes() -> Router<ServerState> {
        Router::new()
            .nest("/room", room::V1::routes())
            .nest("/user", user::V1::routes())
    }
}

use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ServerState {
    pub rooms: Arc<DashMap<RoomId, Arc<Room>>>,
    pub connections: Arc<DashMap<uuid::Uuid, mpsc::Sender<Message>>>,
    pub pcs: Arc<RwLock<HashMap<uuid::Uuid,  Arc<dyn webrtc::peer_connection::PeerConnection>>>>,
    pub pool: Pool,
}


impl ServerState {
    pub fn new(pool: Pool) -> Self {
        Self {
            rooms: Arc::default(),
            connections: Arc::default(),
            pcs: Arc::default(),
            pool,
        }
    }
}

pub async fn run_server(state: ServerState, port: u16) -> std::io::Result<()> {
    let app = Router::new().nest("/v1", ApiV1::routes()).with_state(state);

    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    info!("listen at port {} ...", port);
    axum::serve(listener, app).await
}
