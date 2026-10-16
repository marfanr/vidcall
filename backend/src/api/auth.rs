use std::borrow::Cow;

use argon2::{PasswordHash, PasswordVerifier};
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::StatusCode,
    routing::{get, options, post},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use cookie::time::Duration;
use validator::Validate;

use crate::{
    api::{ApiError, FastJson, db_error, poll_error, server::ServerState},
    utils::get_argon,
};

#[derive(sonic_rs::Deserialize, Validate)]
struct SignInRequest<'a> {
    #[serde(borrow)]
    #[validate(email)]
    email: Cow<'a, str>,

    #[serde(borrow)]
    password: Cow<'a, str>,
}

#[derive(sonic_rs::Serialize)]
struct SignInResponse {
    userid: String,
    token: String,
}

#[derive(Default)]
pub struct V1;

impl V1 {
    #[tracing::instrument()]
    pub fn routes() -> Router<ServerState> {
        Router::new()
            .route("/signin", post(Self::signin))
            .route("/refresh_token", get(Self::refresh_token))
    }

    async fn signin(
        State(state): State<ServerState>,
        jar: CookieJar,
        b: Bytes,
    ) -> Result<(StatusCode, CookieJar, FastJson<SignInResponse>), ApiError> {
        let req: SignInRequest = sonic_rs::from_slice(&b)
            .map_err(|e| ApiError::Rejection(StatusCode::BAD_REQUEST, e.to_string()))?;

        req.validate().map_err(ApiError::Validation)?;

        let db = state.pool.get().await.map_err(poll_error)?;

        let smt = db
            .prepare_cached("SELECT email, password, salt FROM users WHERE email = $1")
            .await
            .map_err(db_error)?;

        let email: &str = req.email.as_ref();
        let Some(user) = db.query_opt(&smt, &[&email]).await.map_err(db_error)? else {
            return Err(ApiError::Rejection(
                StatusCode::UNAUTHORIZED,
                "The credentials you entered are incorrect".into(),
            ));
        };

        let stored_hash: String = user.get("password");
        let parsed_hash = PasswordHash::new(&stored_hash).map_err(|e| {
            tracing::error!("Invalid password hash in database: {e}");
            ApiError::Rejection(
                StatusCode::INTERNAL_SERVER_ERROR,
                "internal server error".into(),
            )
        })?;

        if get_argon()
            .verify_password(req.password.as_bytes(), &parsed_hash)
            .is_err()
        {
            return Err(ApiError::Rejection(
                StatusCode::UNAUTHORIZED,
                "The credentials you entered are incorrect".into(),
            ));
        }

        let cookie = Cookie::build(("refresh_token", "sss"))
            .http_only(true)
            .secure(true)
            .path("/v1/auth")
            .max_age(Duration::days(7))
            .build();

        Ok((
            StatusCode::OK,
            jar.add(cookie),
            FastJson(SignInResponse {
                userid: String::new(),
                token: String::new(),
            }),
        ))
    }

    async fn refresh_token() {}
}