use axum::body::Body;
use axum::extract::Request;
use axum::response::Response;
use http::StatusCode;
use metrics::{counter, describe_counter, describe_gauge, gauge};
use metrics_exporter_prometheus::PrometheusHandle;
use std::net::SocketAddr;
use std::sync::LazyLock;
use std::time::Duration;
use axum::middleware::Next;

static METRICS_BEARER_TOKEN: LazyLock<Option<String>> =
    LazyLock::new(|| std::env::var("PROMETHEUS_TOKEN").ok());

static METRICS_HANDLE: LazyLock<PrometheusHandle> = LazyLock::new(|| {
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener("0.0.0.0:9090".parse::<SocketAddr>().unwrap())
        .install_recorder()
        .unwrap()
});


pub async fn track_metrics(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let is_metrics = path == "/metrics" || path == "/v1/metrics";
    let response = next.run(req).await;
    if !is_metrics {
        increment_requests();
    }
    response
}
pub async fn metrics_handler(req: Request) -> Result<Response, StatusCode> {
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
        .header(
            http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )
        .body(Body::from(body))
        .unwrap())
}

pub fn init_metrics() {
    describe_counter!("vidcall_requests_total", "Total HTTP requests");
    describe_gauge!("vidcall_runtime_threads", "Number of runtime threads");
    describe_gauge!("vidcall_runtime_memory_bytes", "Memory usage in bytes");
    describe_gauge!("vidcall_active_rooms", "Number of active rooms");
    describe_gauge!(
        "vidcall_active_connections",
        "Number of active WebSocket connections"
    );
    describe_gauge!(
        "vidcall_active_peer_connections",
        "Number of active WebRTC peer connections"
    );

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
            } else if line.starts_with("VmRSS:")
                && let Some(val) = line.split_whitespace().nth(1)
            {
                memory_bytes = val.parse::<u64>().unwrap_or(0) * 1024;
            }
        }
    }
    (threads, memory_bytes)
}


fn increment_requests() {
    counter!("vidcall_requests_total").increment(1);
}

// pub fn set_room_count(count: usize) {
//     gauge!("vidcall_active_rooms").set(count as f64);
// }
//
// pub fn set_connection_count(count: usize) {
//     gauge!("vidcall_active_connections").set(count as f64);
// }
//
// pub fn set_peer_connection_count(count: usize) {
//     gauge!("vidcall_active_peer_connections").set(count as f64);
// }