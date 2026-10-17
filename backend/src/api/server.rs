use axum::{
    Router,
    body::Body,
    extract::{Request, ws::Message},
    http::{
        HeaderValue, Method, Response,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    },
};

use dashmap::DashMap;
use deadpool_postgres::Pool;
use tokio::{net::TcpListener, sync::mpsc};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{Level, info};

use crate::{
    api::v1, models::room::{Room, RoomId},
};

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

use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ServerState {
    pub rooms: Arc<DashMap<RoomId, Arc<Room>>>,
    pub connections: Arc<DashMap<uuid::Uuid, mpsc::Sender<Message>>>,
    pub pcs: Arc<RwLock<HashMap<uuid::Uuid, Arc<dyn webrtc::peer_connection::PeerConnection>>>>,
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
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:5173"),
            HeaderValue::from_static("http://127.0.0.1:5173"),
        ])
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([CONTENT_TYPE, AUTHORIZATION, ACCEPT])
        .max_age(Duration::from_secs(3600));

    let app = Router::new()
        .nest("/v1", ApiV1::routes())
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .on_request(|request: &Request<Body>, _span: &tracing::Span| {
                    info!(method = %request.method(), uri = %request.uri(), "request started");
                })
                .on_response(
                    |response: &Response<Body>, latency: Duration, _span: &tracing::Span| {
                        info!(
                            status = %response.status(),
                            latency_ms = latency.as_millis(),
                            "request completed"
                        );
                    },
                ),
        )
        .layer(cors);

    let listener = TcpListener::bind(("0.0.0.0", port)).await?;
    info!("listen at port {} ...", port);
    axum::serve(listener, app).await
}