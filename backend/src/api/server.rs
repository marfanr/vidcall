use axum::{
    body::Body,
    Router,
    extract::{Request, ws::Message},
    http::{
        HeaderValue, Method, StatusCode,
        header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
    },
    middleware::{self, Next},
    response::Response,
};

use dashmap::DashMap;
use deadpool_postgres::Pool;
use metrics::{counter, gauge, describe_counter, describe_gauge};
use metrics_exporter_prometheus::PrometheusHandle;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::{
    api::v1,
    models::room::{Room, RoomId},
};

use std::sync::LazyLock;

static METRICS_BEARER_TOKEN: LazyLock<Option<String>> = LazyLock::new(|| {
    std::env::var("PROMETHEUS_TOKEN").ok()
});

static METRICS_HANDLE: LazyLock<PrometheusHandle> = LazyLock::new(|| {
    let handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener("0.0.0.0:9090".parse::<SocketAddr>().unwrap())
        .install_recorder()
        .unwrap();
    handle
});

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

async fn metrics_handler(req: Request) -> Result<Response, StatusCode> {
    if let Some(ref token) = *METRICS_BEARER_TOKEN {
        let auth_header = req
            .headers()
            .get(http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok());

        match auth_header {
            Some(header) if header.starts_with("Bearer ") => {
                let received = &header[7..];
                if received != token {
                    return Err(StatusCode::UNAUTHORIZED);
                }
            }
            _ => return Err(StatusCode::UNAUTHORIZED),
        }
    }

    let body = METRICS_HANDLE.render();
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(http::header::CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")
        .body(Body::from(body))
        .unwrap())
}

pub fn init_metrics() {
    describe_counter!("vidcall_requests_total", "Total HTTP requests");
    describe_gauge!("vidcall_runtime_threads", "Number of runtime threads");
    describe_gauge!("vidcall_runtime_memory_bytes", "Memory usage in bytes");
    describe_gauge!("vidcall_active_rooms", "Number of active rooms");
    describe_gauge!("vidcall_active_connections", "Number of active WebSocket connections");
    describe_gauge!("vidcall_active_peer_connections", "Number of active WebRTC peer connections");

    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            update_runtime_metrics();
        }
    });
}

pub fn update_runtime_metrics() {
    let (threads, memory_bytes) = get_runtime_metrics();
    gauge!("vidcall_runtime_threads").set(threads as f64);
    gauge!("vidcall_runtime_memory_bytes").set(memory_bytes as f64);
}

fn get_runtime_metrics() -> (u64, u64) {
    let mut threads = 1u64;
    let mut memory_bytes = 0u64;
    if let Ok(content) = std::fs::read_to_string("/proc/self/status") {
        for line in content.lines() {
            if line.starts_with("Threads:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    threads = val.parse().unwrap_or(1);
                }
            } else if line.starts_with("VmRSS:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    memory_bytes = val.parse::<u64>().unwrap_or(0) * 1024;
                }
            }
        }
    }
    (threads, memory_bytes)
}

pub fn increment_requests() {
    counter!("vidcall_requests_total").increment(1);
}

pub fn set_room_count(count: usize) {
    gauge!("vidcall_active_rooms").set(count as f64);
}

pub fn set_connection_count(count: usize) {
    gauge!("vidcall_active_connections").set(count as f64);
}

pub fn set_peer_connection_count(count: usize) {
    gauge!("vidcall_active_peer_connections").set(count as f64);
}

#[derive(Clone)]
pub struct ServerState {
    pub rooms: Arc<DashMap<RoomId, Arc<Room>>>,
    pub connections: Arc<DashMap<uuid::Uuid, mpsc::Sender<Message>>>,
    pub pcs: Arc<DashMap<uuid::Uuid, Arc<dyn webrtc::peer_connection::PeerConnection>>>,
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

async fn track_metrics(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let is_metrics = path == "/metrics" || path == "/v1/metrics";
    let response = next.run(req).await;
    if !is_metrics {
        increment_requests();
    }
    response
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
