use axum::{
    Router,
    extract::ws::Message,
    http::{
        HeaderValue, Method,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    },
    middleware::{self},
};

use dashmap::DashMap;
use deadpool_postgres::Pool;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::api::metrics::{metrics_handler, track_metrics};
use crate::api::v1;

#[derive(Default)]
struct ApiV1 {}

impl ApiV1 {
    fn routes() -> Router<ServerState> {
        Router::new()
            .nest("/room", v1::room::routes())
            .nest("/user", v1::user::routes())
            .nest("/auth", v1::auth::routes())
    }
}

#[derive(Clone)]
pub struct ServerState {
    // pub rooms: Arc<DashMap<RoomId, Arc<Room>>>,
    pub connections: Arc<DashMap<uuid::Uuid, mpsc::Sender<Message>>>,
    pub pcs: Arc<DashMap<uuid::Uuid, Arc<dyn webrtc::peer_connection::PeerConnection>>>,
    pub pool: Pool,
}

impl ServerState {
    pub fn new(pool: Pool) -> Self {
        Self {
            // rooms: Arc::default(),
            connections: Arc::default(),
            pcs: Arc::default(),
            pool,
        }
    }
}

pub async fn run_server(state: ServerState, port: u16) -> std::io::Result<()> {
    let cors = CorsLayer::new()
        // TODO: add BASE URL on .env
        .allow_origin([
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://127.0.0.1:5173"),
        ])
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION, ACCEPT])
        .max_age(Duration::from_secs(3600));

    let app = Router::new()
        .route("/metrics", axum::routing::get(metrics_handler))
        .nest("/v1", ApiV1::routes())
        .with_state(state)
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(track_metrics))
        .layer(cors);

    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    info!("listen at port {} ...", port);
    axum::serve(listener, app).await
}
