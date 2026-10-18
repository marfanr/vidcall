use chrono::Utc;
use reqwest::Client;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::{timeout, Duration};

static LOG_CHANNEL: std::sync::LazyLock<Arc<mpsc::Sender<LogEntry>>> = std::sync::LazyLock::new(|| {
    let (tx, rx) = mpsc::channel(1000);
    std::thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                loki_worker(rx).await;
            });
    });
    Arc::new(tx)
});

#[derive(Clone, Debug, Serialize)]
struct LogEntry {
    message: String,
    level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    room_id: Option<String>,
}

#[derive(Serialize)]
struct LokiStream {
    stream: LogLabels,
    values: Vec<[String; 2]>,
}

#[derive(Serialize)]
struct LogLabels {
    service: String,
    level: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    user_id: Option<String>,
}

#[derive(Serialize)]
struct LokiPayload {
    streams: Vec<LokiStream>,
}

async fn loki_worker(mut rx: mpsc::Receiver<LogEntry>) {
    let client = Client::new();
    let endpoint = std::env::var("LOKI_ENDPOINT").unwrap_or_else(|_| "http://localhost:3100/loki/api/v1/push".to_string());

    loop {
        let mut batch = Vec::new();
        match timeout(Duration::from_millis(500), rx.recv()).await {
            Ok(Some(entry)) => {
                batch.push(entry);
                while let Ok(entry) = rx.try_recv() {
                    batch.push(entry);
                    if batch.len() >= 100 {
                        break;
                    }
                }
            }
            _ => continue,
        }

        if batch.is_empty() {
            continue;
        }

        let mut streams: std::collections::HashMap<String, Vec<LogEntry>> = std::collections::HashMap::new();
        for entry in batch {
            let key = format!("{}:{}", entry.level, entry.user_id.as_deref().unwrap_or(""));
            streams.entry(key).or_default().push(entry);
        }

        let loki_streams: Vec<LokiStream> = streams.into_iter().map(|(_, entries)| {
            let level = entries.first().map(|e| e.level.clone()).unwrap_or_default();
            let user_id = entries.first().and_then(|e| e.user_id.clone());

            let values: Vec<[String; 2]> = entries.into_iter().map(|e| {
                let ts = Utc::now().timestamp_nanos_opt().unwrap_or(0).to_string();
                let msg = serde_json::json!({
                    "message": e.message,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "user_id": e.user_id,
                    "room_id": e.room_id,
                }).to_string();
                [ts, msg]
            }).collect();

            LokiStream {
                stream: LogLabels {
                    service: "vidcall-backend".to_string(),
                    level,
                    user_id,
                },
                values,
            }
        }).collect();

        let payload = LokiPayload { streams: loki_streams };

        if let Err(e) = client.post(&endpoint)
            .json(&payload)
            .send()
            .await
        {
            eprintln!("Failed to send logs to Loki: {}", e);
        }
    }
}

pub fn loki_log(level: &str, message: &str) {
    let entry = LogEntry {
        message: message.to_string(),
        level: level.to_lowercase(),
        user_id: None,
        room_id: None,
    };
    if let Err(e) = LOG_CHANNEL.try_send(entry) {
        eprintln!("Failed to send log: {}", e);
    }
}

pub fn loki_log_with_context(level: &str, message: &str, user_id: Option<String>, room_id: Option<String>) {
    let entry = LogEntry {
        message: message.to_string(),
        level: level.to_lowercase(),
        user_id,
        room_id,
    };
    if let Err(e) = LOG_CHANNEL.try_send(entry) {
        eprintln!("Failed to send log: {}", e);
    }
}

pub fn init_logger() {
    let _ = LOG_CHANNEL.clone();
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => ({
        let msg = format!($($arg)*);
        $crate::logger::loki_log("info", &msg);
    });
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => ({
        let msg = format!($($arg)*);
        $crate::logger::loki_log("error", &msg);
    });
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => ({
        let msg = format!($($arg)*);
        $crate::logger::loki_log("warn", &msg);
    });
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => ({
        let msg = format!($($arg)*);
        $crate::logger::loki_log("debug", &msg);
    });
}
