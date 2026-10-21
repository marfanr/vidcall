use axum::body::Bytes;
use axum::extract::Path;
use axum::{
    Extension, Router,
    extract::State,
    http::StatusCode,
    middleware,
    routing::{get, post},
};
use std::borrow::Cow;

use sonic_rs::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

use crate::api::server::bearer_middleware;
use crate::api::{
    ApiError::{self},
    FastJson, poll_error,
    server::ServerState,
};
use crate::models;
use crate::models::StandardModel;
use crate::utils::{AccessClaims, validate_uuid};

#[derive(Deserialize, Validate)]
struct CreateRoomReq<'a> {
    #[validate(length(min = 1))]
    #[serde(borrow)]
    alias: Cow<'a, str>,
    #[validate(custom(function = "validate_uuid"))]
    #[serde(borrow)]
    owner_id: Cow<'a, str>,
    #[serde(borrow)]
    passcode: Option<Cow<'a, str>>,
    #[validate(length(min = 1))]
    #[serde(borrow)]
    options: Cow<'a, str>,
}

#[derive(Serialize)]
struct CreateRoomRes {
    id: String,
}

#[derive(Serialize)]
struct GetRoomRes {
    id: Uuid,
    owner: String,
}

pub fn routes() -> Router<ServerState> {
    Router::new()
        .route("/", post(self::create_room))
        .route("/{id}", get(self::get_room))
        .layer(middleware::from_fn(bearer_middleware))
}

#[tracing::instrument(skip(state))]
async fn create_room(
    Extension(claims): Extension<AccessClaims>,
    State(state): State<ServerState>,
    b: Bytes,
) -> Result<(StatusCode, FastJson<CreateRoomRes>), ApiError> {
    let req = sonic_rs::from_slice::<CreateRoomReq>(&b).map_err(|e| {
        ApiError::Rejection(
            StatusCode::BAD_REQUEST,
            format!("parse json: {e}").to_string(),
        )
    })?;

    req.validate().map_err(ApiError::Validation)?;

    let user_id = Uuid::parse_str(claims.sub.as_str())
        .map_err(|_| ApiError::Rejection(StatusCode::BAD_REQUEST, "invalid user id".to_string()))?;

    let db = state.pool.get().await.map_err(poll_error)?;

    let room = models::room::Room::new(&req.alias, user_id, req.passcode.as_deref());
    let room_id = room.create(&db).await?.to_string();

    Ok((StatusCode::OK, FastJson(CreateRoomRes { id: room_id })))
}

#[tracing::instrument(skip(state))]
async fn get_room(
    State(state): State<ServerState>,
    Path(p): Path<String>,
) -> Result<(StatusCode, FastJson<GetRoomRes>), ApiError> {
    let db = state.pool.get().await.map_err(poll_error)?;
    let room = models::room::Room::get_by_alias(&db, p.as_str()).await?;
    let room_id = room.id.expect("room id must be provided");
    let user = models::user::User::get_by_id(&db, room.owner_id).await?;

    let username : &str = &user.username;

    Ok((
        StatusCode::OK,
        FastJson(GetRoomRes {
            id: room_id,
            owner: user.username.to_string(),
        }),
    ))
}
