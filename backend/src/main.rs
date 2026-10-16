use std::{
    env,
    str::FromStr,
    sync::{Arc, RwLock},
};

use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::{
    api::server::{ServerState, run_server},
    models::{
        conn,
        room::{Room, RoomId},
    },
};

pub mod api;
pub mod models;
pub mod sfu;
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
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_env("LOG").unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let pool = conn::init_db().await?;
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

    run_server(state, 8080).await?;
    Ok(())
}



#[global_allocator]
static GLOBAL: jemallocator::Jemalloc = jemallocator::Jemalloc;