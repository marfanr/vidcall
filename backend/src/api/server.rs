use axum::{
    Router,
    body::Body,
    extract::ws::Message,
    http::{
        HeaderValue, Method,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    },
    middleware::{self, Next},
};

use axum::extract::Request;
use axum::response::{IntoResponse, Response};
use axum_extra::TypedHeader;
use dashmap::DashMap;
use deadpool_postgres::Pool;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, mpsc};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::api::metrics::{metrics_handler, track_metrics};
use crate::api::{ApiError, v1};
use crate::sfu::id::RoomId;
use crate::sfu::room::RoomSfu;
use crate::utils::{AccessClaims, verify_jwt};
use headers::{Authorization, authorization::Bearer};
use http::StatusCode;

#[derive(Default)]
struct ApiV1 {}

impl ApiV1 {
    fn routes() -> Router<ServerState> {
        Router::new()
            .nest("/room", v1::room::routes())
            .nest("/user", v1::user::routes())
            .nest("/auth", v1::auth::routes())
            .nest("/ws", v1::ws::routes())
    }
}

#[derive(Clone)]
pub struct ServerState {
    pub rooms: Arc<DashMap<uuid::Uuid, Arc<RoomSfu>>>,
    pub wsconn: Arc<DashMap<uuid::Uuid, mpsc::Sender<Message>>>,
    pub pool: Pool,
    pub hash_sem: Arc<Semaphore>,
    pub allowed_origins: Arc<Vec<String>>,
}

impl ServerState {
    pub fn new(pool: Pool, allowed_origins: Vec<String>) -> Self {
        Self {
            rooms: Arc::default(),
            wsconn: Arc::default(),
            pool,
            hash_sem: Arc::new(Semaphore::new(4)),
            allowed_origins: Arc::new(allowed_origins),
        }
    }
}

pub async fn run_server(state: ServerState, port: u16) -> std::io::Result<()> {
    let cors_origins: Vec<HeaderValue> = state
        .allowed_origins
        .iter()
        .map(|o| HeaderValue::from_str(o).expect("invalid origin"))
        .collect();

    let cors = CorsLayer::new()
        // TODO: add BASE URL on .env
        .allow_origin(cors_origins)
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

pub async fn bearer_middleware(
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let Ok(claims) = verify_jwt::<AccessClaims>(auth.token()) else {
        return ApiError::Rejection(StatusCode::UNAUTHORIZED, "invalid bearer token".to_owned())
            .into_response();
    };

    req.extensions_mut().insert(claims);

    next.run(req).await
}
