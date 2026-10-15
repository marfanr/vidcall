use axum::{
    Router,
    extract::ws::{Message},
    routing::{get, post},
};

use deadpool_postgres::Pool;
use tokio::{net::TcpListener, sync::mpsc};
use tracing::info;

use crate::{api::room, models::room::Room};

#[derive(Default)]
struct ApiV1 {}

impl ApiV1 {
    fn routes() -> Router<ServerState> {
        Router::new()
            .route("/room", post(room::V1::create_room))
            .route("/room", get(room::V1::get_room))
            .route("/room/ws", get(room::V1::ws))
    }
}

use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

#[derive(Clone, Default)]
pub struct ServerState {
    pub rooms: Arc<RwLock<Vec<Room>>>,
    pub connections: Arc<RwLock<HashMap<uuid::Uuid, mpsc::Sender<Message>>>>,
    pub pcs: Arc<RwLock<HashMap<uuid::Uuid,  Arc<dyn webrtc::peer_connection::PeerConnection>>>>,
}

#[derive(Default, Clone)]
pub struct ApiServer {
    pool: Option<Pool>,
    state: Option<ServerState>,
}

impl ApiServer {
    pub fn pool(&mut self, pool: Pool) {
        self.pool = Some(pool);
    }

    pub fn state(&mut self, state: ServerState) {
        self.state = Some(state);
    }

    pub async fn serve(&self, port: u16) -> std::io::Result<()> {
        let state = self.state.clone().unwrap_or_default();
        let app = Router::new().nest("/v1", ApiV1::routes()).with_state(state);

        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        info!("listen at port {} ...", port);
        axum::serve(listener, app).await
    }
}
