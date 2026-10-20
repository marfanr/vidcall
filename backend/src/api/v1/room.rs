use axum::body::Bytes;
use axum::{
    Router,
    extract::State,
    http::StatusCode,
    routing::{get, post},
};
use axum_auth::AuthBearer;
use sonic_rs::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::api::v1::ws::ws;
use crate::api::{
    ApiError::{self},
    FastJson, poll_error,
    server::ServerState,
};
use crate::models;
use crate::models::StandardModel;
use crate::utils::{AccessClaims, validate_uuid, verify_jwt};

#[derive(Deserialize, Validate)]
struct CreateRoomReq {
    #[validate(length(min = 1))]
    alias: String,
    #[validate(custom(function = "validate_uuid"))]
    owner_id: String,
    passcode: String,
    #[validate(length(min = 1))]
    options: String,
}

#[derive(Serialize)]
struct CreateRoomRes {
    token: Option<String>,
}

pub fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", post(self::create_room))
        .route("/ws", get(ws))
}

#[tracing::instrument(skip(state, token))]
async fn create_room(
    State(state): State<ServerState>,
    AuthBearer(token): AuthBearer,
    b: Bytes,
) -> Result<(StatusCode, FastJson<CreateRoomRes>), ApiError> {
    let req = sonic_rs::from_slice::<CreateRoomReq>(&b).map_err(|e| {
        ApiError::Rejection(
            StatusCode::BAD_REQUEST,
            format!("parse json: {e}").to_string(),
        )
    })?;

    req.validate().map_err(ApiError::Validation)?;

    let Ok(claims) = verify_jwt::<AccessClaims>(token.as_str()) else {
        return Err(ApiError::Rejection(
            StatusCode::UNAUTHORIZED,
            "invalid bearer token".to_owned(),
        ));
    };

    println!("claims: {:?}", &claims);

    let user_id = Uuid::parse_str(claims.sub.as_str())
        .map_err(|_| ApiError::Rejection(StatusCode::BAD_REQUEST, "invalid user id".to_string()))?;

    let db = state.pool.get().await.map_err(poll_error)?;
    let room = models::room::Room::new(&req.alias, user_id, &req.passcode);
    let _rom_id = room.create(db).await?;

    Ok((StatusCode::OK, FastJson(CreateRoomRes { token: None })))
}
