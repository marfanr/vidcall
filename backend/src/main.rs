use std::{env};

use tracing::info;

use crate::models::conn;

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

    conn::init_db().await?;

    let args = handle_args();
    if args.is_running_migration {
        // TODO: create migration
        info!("run migrations");
        return Ok(());
    }
    let server = api::server::ApiServer::default();

    server.serve(8080).await?;

    Ok(())
}
