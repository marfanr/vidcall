use std::{env, sync::Arc};

use tracing::info;
use uuid::Uuid;

use crate::{api::server::ServerState, models::conn};

pub mod api;
pub mod models;
pub mod sfu;

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
        .with_max_level(tracing::Level::INFO)
        .init();

    let pool = conn::init_db().await?;
    let state = ServerState::default();

    let args = handle_args();
    if args.is_running_migration {
        // TODO: create migration
        info!("run migrations");
        return Ok(());
    }
    
    {
        let mut rooms = state.rooms.write().await;
       
        rooms.push(crate::models::room::Room {
            id: Arc::from(String::from("01a132ad-21dd-745c-96ef-322546ed33be")),
            name: Arc::from("default"),
            connection: Vec::new(),
        });
        info!("rooms : {:?}", &rooms);
    }

    let mut server = api::server::ApiServer::default();
    server.pool(pool);
    server.state(state);

    
    server.serve(8080).await?;
    Ok(())
}
