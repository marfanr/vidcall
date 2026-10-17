use std::{
    borrow::Cow,
    time::{SystemTime, UNIX_EPOCH},
};

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
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{ApiError, FastJson, db_error, poll_error, server::ServerState}, utils::{AuthClaims, create_jwt, get_argon, verify_jwt},
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
    token: String,
}

#[derive(Default)]
pub struct V1;

impl V1 {
    #[tracing::instrument()]
    pub fn routes() -> Router<ServerState> {
        Router::new()
            .route("/signin", post(Self::signin))
            .route("/refresh", post(Self::refresh_token))
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
            .prepare_cached("SELECT id, email, password, salt FROM users WHERE email = $1")
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

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock before epoch")
            .as_secs();

        let userid: Uuid = user.get("id");

        let refresh_jws = create_jwt::<AuthClaims>(&AuthClaims {
            iat: now,
            sub: req.email.to_string(),
            exp: now + (30 * 24 * 60 * 60),
            id: userid.to_string(),
            ..Default::default()
        })
        .expect("failed create jwt");

        let token_jws = create_jwt::<AuthClaims>(&AuthClaims {
            iat: now,
            sub: req.email.to_string(),
            exp: now + (7 * 24 * 60 * 60),
            id: userid.to_string(),
            ..Default::default()
        })
        .expect("failed create jwt");

        let refresh_token = format!(
            "{}.{}.{}",
            refresh_jws.protected, refresh_jws.payload, refresh_jws.signature
        );
        let token = format!(
            "{}.{}.{}",
            token_jws.protected, token_jws.payload, token_jws.signature
        );

        let cookie = Cookie::build(("refresh_token", refresh_token))
            .http_only(true)
            .secure(true)
            .path("/")
            .max_age(Duration::days(7))
            .build();

        Ok((
            StatusCode::OK,
            jar.add(cookie),
            FastJson(SignInResponse { token: token }),
        ))
    }

    async fn refresh_token(State(state): State<ServerState>, jar: CookieJar) {
        if let Some(refresh) = jar.get("refresh_token") {
            info!("refresh token {}", refresh);
            if let Ok(claims) = verify_jwt::<AuthClaims>(refresh.value()) {
                info!("claims {:#?}", claims);
            }

        };
    }
}
