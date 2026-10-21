use std::borrow::Cow;

use axum::{Router, body::Bytes, extract::State, http::StatusCode, routing::post};
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{ApiError, FastJson, poll_error, server::ServerState},
    models::user,
};
use crate::models::StandardModel;

#[derive(sonic_rs::Deserialize, Validate)]
struct UserRequest<'a> {
    #[serde(borrow)]
    #[validate(length(min = 3, max = 32))]
    username: Cow<'a, str>,

    #[serde(borrow)]
    #[validate(email)]
    email: Cow<'a, str>,

    #[serde(borrow)]
    password: Cow<'a, str>,
}

#[derive(sonic_rs::Serialize)]
struct UserResponse {
    userid: Uuid,
}

pub fn routes() -> Router<ServerState> {
    Router::new().route("/", post(self::create_new_user))
}

async fn create_new_user(
    State(state): State<ServerState>,
    b: Bytes,
) -> Result<(StatusCode, FastJson<UserResponse>), ApiError> {
    let req: UserRequest = sonic_rs::from_slice(&b)
        .map_err(|e| ApiError::Rejection(StatusCode::BAD_REQUEST, e.to_string()))?;

    req.validate().map_err(ApiError::Validation)?;

    let db = state.pool.get().await.map_err(poll_error)?;
    let user = user::User::new(&req.email, &req.username, &req.password);
    let id = user.create(&db).await?;

    Ok((StatusCode::OK, FastJson(UserResponse { userid: id })))
}
