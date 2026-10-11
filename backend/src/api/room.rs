// create room
use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::api::{ApiError, ValidateJson};

#[derive(Deserialize, Validate)]
pub struct CreateRoomReq {
    #[validate(length(min = 1, max = 64))]
    name: String,
}

#[derive(Serialize)]
pub struct CreateRoomRes {
    token: Option<&'static str>,
}

#[derive(Default)]
pub struct V1 {}

impl V1 {
    pub async fn create_room(
        ValidateJson(r_): ValidateJson<CreateRoomReq>,
    ) -> Result<(StatusCode, Json<CreateRoomRes>), ApiError> {
        Ok((StatusCode::OK, Json(CreateRoomRes { token: None })))
    }
}
