// create room
use axum::Json;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::api::ApiError;

#[derive(Deserialize)]
pub struct CreateRoomReq {
    name: Option<String>,
}

#[derive(Serialize)]
pub struct CreateRoomRes {
    token: Option<&'static str>
}

#[derive(Default)]
pub struct V1 {}

impl V1 {
    pub async fn create_room(Json(req): Json<CreateRoomReq>) -> 
    Result<Json<CreateRoomRes>, ApiError> {
        if let Some(n) = req.name {
            info!("req: {}", n);
        }
        Err(ApiError::NotFound)
        // Ok(Json(CreateRoomRes {
        //     token: None,
        // }))
    }
}