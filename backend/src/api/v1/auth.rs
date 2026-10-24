use std::borrow::Cow;

use argon2::{PasswordHash, PasswordVerifier};
use axum::{Router, body::Bytes, extract::State, http::StatusCode, routing::post};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use chrono::Utc;
use cookie::time::Duration as CookieDuration;
use tracing::Instrument;
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use crate::{
    api::{ApiError, FastJson, poll_error, server::ServerState},
    models::{refresh_tokens as rt, user},
    utils::{FAMILY_TTL, REFRESH_TTL, RefreshClaims, get_argon, sign_pair, verify_jwt},
};
use crate::models::ModelError;

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

pub fn routes() -> Router<ServerState> {
    Router::new()
        .route("/signin", post(self::signin))
        .route("/refresh", post(self::refresh_token))
        .route("/logout", post(self::logout))
}

#[tracing::instrument(skip_all)]
async fn signin(
    State(state): State<ServerState>,
    jar: CookieJar,
    b: Bytes,
) -> Result<(StatusCode, CookieJar, FastJson<SignInResponse>), ApiError> {
    let req: SignInRequest = sonic_rs::from_slice(&b)
        .map_err(|e| ApiError::Rejection(StatusCode::BAD_REQUEST, e.to_string()))?;

    req.validate().map_err(ApiError::Validation)?;

    let db = state
        .pool
        .get()
        .instrument(tracing::info_span!("db.pool_get"))
        .await
        .map_err(poll_error)?;

    let email: &str = req.email.as_ref();
    let found_user = user::User::get_by_email(&db, email).await?;

    let password_bytes = req.password.as_bytes().to_vec();
    let hash_string = found_user.password;

    let span = tracing::info_span!("argon.verify");

    let permit = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        state.hash_sem.clone().acquire_owned(),
    )
    .await
    .map_err(|_| ApiError::Rejection(StatusCode::SERVICE_UNAVAILABLE, "busy, try again".into()))?
    .map_err(|_| {
        ApiError::Rejection(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error".into(),
        )
    })?;

    let is_valid = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        span.in_scope(|| {
            let Ok(parsed_hash) = PasswordHash::new(&hash_string) else {
                return false;
            };
            get_argon()
                .verify_password(&password_bytes, &parsed_hash)
                .is_ok()
        })
    })
    .await
    .map_err(|e| {
        tracing::error!("spawn_blocking error in password verification: {e}");
        ApiError::Rejection(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error".into(),
        )
    })?;

    if !is_valid {
        return Err(ApiError::Rejection(
            StatusCode::UNAUTHORIZED,
            "The credentials you entered are incorrect".into(),
        ));
    }

    let user_id = found_user.id.expect("user_id should not be None");

    let jti = Uuid::now_v7();
    let family_id = Uuid::now_v7();

    let sign = sign_pair(user_id, "api", jti, family_id).expect("failed to sign");

    let now = Utc::now();
    let family_expires_at = now + FAMILY_TTL;
    let expires_at = (now + REFRESH_TTL).min(family_expires_at);

    rt::publish(&db, user_id, jti, family_id, expires_at, family_expires_at).await?;

    let cookie = Cookie::build(("refresh_token", sign.refresh_token))
        .http_only(true)
        .secure(true)
        .same_site(cookie::SameSite::None)
        .path("/v1/auth/refresh")
        .max_age(CookieDuration::days(7))
        .build();

    Ok((
        StatusCode::OK,
        jar.add(cookie),
        FastJson(SignInResponse {
            token: sign.access_token,
        }),
    ))
}

#[tracing::instrument(skip_all)]
async fn refresh_token(
    State(state): State<ServerState>,
    jar: CookieJar,
) -> Result<(StatusCode, CookieJar, FastJson<SignInResponse>), ApiError> {
    let Some(refresh) = jar.get("refresh_token") else {
        return Err(ApiError::Rejection(
            StatusCode::OK,
            "no refresh_token found".to_string(),
        ));
    };

    let claims = verify_jwt::<RefreshClaims>(refresh.value()).map_err(|e| {
        info!("invalid jwt: {}", e);
        ApiError::NotFound
    })?;

    let db = state
        .pool
        .get()
        .instrument(tracing::info_span!("db.pool_get"))
        .await
        .map_err(poll_error)?;

    let jti: Uuid = claims
        .jti
        .parse()
        .map_err(|_| ApiError::Rejection(StatusCode::BAD_REQUEST, "invalid jti".into()))?;
    let family_id: Uuid = claims
        .family_id
        .parse()
        .map_err(|_| ApiError::Rejection(StatusCode::BAD_REQUEST, "invalid family_id".into()))?;
    let user_id: Uuid = claims
        .sub
        .parse()
        .map_err(|_| ApiError::Rejection(StatusCode::BAD_REQUEST, "invalid user_id".into()))?;

    let now = Utc::now();

    let token = match rt::find_and_mark_used(&db, jti, now).await {
        Ok(token) => token,
        Err(_) => {
            let reused = rt::check_reuse_and_revoke_family(&db, jti, family_id).await?;
            if reused {
                return Err(ApiError::NotFound);
            }
            return Err(ApiError::NotFound);
        }
    };

    let new_jti = Uuid::now_v7();
    let new_expires_at = (now + REFRESH_TTL).min(token.family_expires_at);

    let sign = sign_pair(user_id, "api", new_jti, family_id).map_err(|e| {
        tracing::error!("failed to sign new token pair: {}", e);
        ApiError::Rejection(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal server error".into(),
        )
    })?;

    rt::publish(
        &db,
        user_id,
        new_jti,
        family_id,
        new_expires_at,
        token.family_expires_at,
    )
    .await?;

    let cookie = Cookie::build(("refresh_token", sign.refresh_token))
        .http_only(true)
        .secure(true)
        .same_site(cookie::SameSite::None)
        .path("/v1/auth/refresh")
        .max_age(CookieDuration::days(7))
        .build();

    Ok((
        StatusCode::OK,
        jar.add(cookie),
        FastJson(SignInResponse {
            token: sign.access_token,
        }),
    ))
}

async fn logout(
    State(state): State<ServerState>,
    jar: CookieJar,
) -> Result<(StatusCode, CookieJar), ApiError> {
    let Some(refresh) = jar.get("refresh_token") else {
        return Err(ApiError::NotFound);
    };

    let claims = verify_jwt::<RefreshClaims>(refresh.value()).map_err(|e| {
        info!("invalid jwt: {}", e);
        ApiError::NotFound
    })?;

    let family_id: Uuid = claims
        .family_id
        .parse()
        .map_err(|_| ApiError::Rejection(StatusCode::BAD_REQUEST, "invalid family_id".into()))?;

    let db = state.pool.get().await.map_err(poll_error)?;

    rt::revoke_family(&db, family_id).await?;

    let cookie = Cookie::build(("refresh_token", ""))
        .http_only(true)
        .secure(true)
        .same_site(cookie::SameSite::None)
        .path("/v1/auth/refresh")
        .max_age(CookieDuration::seconds(0))
        .build();

    Ok((StatusCode::OK, jar.add(cookie)))
}
