use axum::{Router, routing::post};

use tokio::net::TcpListener;
use tracing::info;

use crate::api::room;

#[derive(Default)]
struct ApiV1 {}

impl ApiV1 {
    fn routes() -> Router {
        Router::new().route("/room", post(room::V1::create_room))
    }
}

#[derive(Default)]
pub struct ApiServer {}

impl ApiServer {
    pub async fn serve(&self, port: u16) -> std::io::Result<()> {
        let app = Router::new().nest("/v1", ApiV1::routes());

        let listener = TcpListener::bind(("0.0.0.0", port)).await?;
        info!("listen at port {} ...", port);
        axum::serve(listener, app).await
    }
}
