use std::{borrow::Cow, fmt::Display, str::FromStr, sync::Arc};

use crate::api::{ApiError, FastJson, server::ServerState};
use argon2::{Algorithm, Argon2, Params, PasswordHasher, Version, password_hash::phc::SaltString};
use axum::{Router, body::Bytes, extract::State, http::StatusCode, routing::post};
use tokio_postgres::error::SqlState;
use tracing::{error, trace};
use validator::Validate;

#[derive(sonic_rs::Deserialize, Validate)]
struct UserRequest<'a> {
    #[serde(borrow)]
    #[validate(length(min = 3, max = 32))]
    username: Cow<'a, str>,

    #[serde(borrow)]
    email: Cow<'a, str>,

    #[serde(borrow)]
    password: Cow<'a, str>,
}

#[derive(sonic_rs::Serialize)]
struct UserResponse {
    userid: String,
}

#[derive(Default)]
pub struct V1;

fn unavailable(e: impl Display) -> ApiError {
    ApiError::Rejection(
        StatusCode::SERVICE_UNAVAILABLE,
        "service unavailable".to_string(),
    )
}

fn poll_error(e: impl Display) -> ApiError {
    error!("db pool: {e}");
    ApiError::Rejection(
        StatusCode::SERVICE_UNAVAILABLE,
        "service unavailable".to_string(),
    )
}

fn db_error(e: tokio_postgres::Error) -> ApiError {
    match e.as_db_error() {
        Some(db) => error!(
            sqlstate = db.code().code(),
            message = db.message(),
            detail = ?db.detail(),
            table = ?db.table(),
            column = ?db.column(),
            constraint = ?db.constraint(),
            "db error"
        ),
        // bukan error dari server (koneksi putus, serialisasi parameter, dll.)
        None => error!(error = ?e, source = ?std::error::Error::source(&e), "db error"),
    }

    unavailable(e)
}

impl V1 {
    pub fn routes() -> Router<ServerState> {
        Router::new().route("/", post(Self::create_new_user))
    }

    async fn create_new_user(
        State(_state): State<ServerState>,
        b: Bytes,
    ) -> Result<(StatusCode, FastJson<UserResponse>), ApiError> {
        let req: UserRequest = sonic_rs::from_slice(&b)
            .map_err(|e| ApiError::Rejection(StatusCode::BAD_REQUEST, e.to_string()))?;

        req.validate().map_err(ApiError::Validation)?;

        // todo: move this
        let argon2 = Argon2::new_with_secret(
            b"haiya",
            Algorithm::Argon2id,
            Version::V0x13,
            Params::default(),
        )
        .expect("error to setup argon");

        let db = _state.pool.get().await.map_err(poll_error)?;
        let id = uuid::Uuid::now_v7();

        let salt = SaltString::generate();

        let pw = argon2
            .hash_password_with_salt(req.password.as_bytes(), &salt.to_salt())
            .unwrap()
            .to_string();

        let stmt = db
            .prepare_cached(
                "INSERT INTO users (id, username, email, password, salt) VALUES ($1, $2, $3, $4, $5)",
            )
            .await
            .map_err(db_error)?;

        let res = db
            .execute(
                &stmt,
                &[&id, &req.username, &req.email, &pw, &salt.to_string()],
            )
            .await
            .map_err(|e| {
                let Some(db) = e.as_db_error() else {
                    return db_error(e);
                };

                if *db.code() == SqlState::UNIQUE_VIOLATION {
                    return match db.constraint() {
                        Some("users_username_key") => {
                            ApiError::Rejection(StatusCode::CONFLICT, "users already exists".into())
                        }
                        _ => ApiError::Rejection(StatusCode::CONFLICT, "already exists".into()),
                    };
                }

                db_error(e)
            })?;

        trace!("create_new_users: affected {} rows", res);

        Ok((
            StatusCode::OK,
            FastJson(UserResponse {
                userid: id.to_string(),
            }),
        ))
    }
}
