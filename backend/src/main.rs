use std::{
    env,
    str::FromStr,
    sync::{Arc, RwLock},
};

use tracing::info;

use crate::{
    api::server::{ServerState, init_metrics, run_server},
    models::{
        conn,
        room::{Room, RoomId},
    },
    utils::init_keys,
};

fn init_rustls() {
    let provider = rustls::crypto::ring::default_provider();
    let _ = provider.install_default();
}

pub mod api;
pub mod db;
pub mod logger;
pub mod models;
pub mod sfu;
pub mod telemetry;
pub mod utils;

#[derive(Default)]
struct ArgsParsed {
    is_running_migration: bool,
}

fn handle_args() -> ArgsParsed {
    let mut parsed = ArgsParsed::default();
    let args: Vec<String> = env::args().collect();
    for item in &args {
        if item.contains("migration") {
            parsed.is_running_migration = true;
        }
    }
    parsed
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_rustls();
    dotenvy::dotenv().ok();

    if let Err(e) = telemetry::init_telemetry() {
        eprintln!("Failed to initialize telemetry: {}", e);
    }

    logger::init_logger();

    init_keys();
    let pool = conn::init_db()?;
    let state = ServerState::new(pool);

    let args = handle_args();
    if args.is_running_migration {
        // TODO: create migration
        info!("run migrations");
        return Ok(());
    }

    // ONLY FOR DEBUGGING PURPOSE
    // will be removed later
    {
        let roomid = RoomId::from_str("blWGZU0eG9h0").unwrap();
        info!("room id : {}", roomid);
        state.rooms.insert(
            roomid.clone(),
            Arc::from(Room::new(roomid, RwLock::new(Arc::from("default")))),
        );
    }

    init_metrics();
    run_server(state, 8080).await?;

    telemetry::shutdown_telemetry();
    Ok(())
}

#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;
