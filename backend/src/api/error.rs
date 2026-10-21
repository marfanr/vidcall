use std::fmt::Display;
use std::sync::Arc;

use axum::{Json, http::StatusCode, response::IntoResponse};
use tracing::error;
use validator::ValidationErrors;

use crate::models::ModelError;

// Error response
#[derive(Debug)]
pub enum ApiError {
    NotFound,
    Validation(ValidationErrors),
    Rejection(StatusCode, String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            Self::NotFound => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(sonic_rs::json!({
                    "error": "internal_error"
                })),
            )
                .into_response(),

            Self::Rejection(status, msg) => (
                status,
                Json(sonic_rs::json!({
                    "error": msg
                })),
            )
                .into_response(),

            Self::Validation(val) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(sonic_rs::json!({
                    "error": "validation fields",
                    "fields": val,
                })),
            )
                .into_response(),
        }
    }
}

impl From<ModelError> for ApiError {
    fn from(e: ModelError) -> Self {
        match e {
            ModelError::NotFound(msg) => Self::Rejection(StatusCode::NOT_FOUND, msg),
            ModelError::Conflict(msg) => Self::Rejection(StatusCode::CONFLICT, msg),
            ModelError::Unprocessable(msg) => {
                Self::Rejection(StatusCode::UNPROCESSABLE_ENTITY, msg)
            }
            ModelError::Database(e) => db_error(e),
            ModelError::Internal(msg) => {
                error!("internal model error: {msg}");
                Self::Rejection(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal server error".to_owned(),
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct WsError(pub(crate) Arc<str>);

impl std::fmt::Display for WsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for WsError {
    fn from(s: String) -> Self {
        Self(Arc::from(s))
    }
}

pub fn unavailable(_e: impl Display) -> ApiError {
    ApiError::Rejection(
        StatusCode::SERVICE_UNAVAILABLE,
        "service unavailable".to_string(),
    )
}

pub fn poll_error(e: impl Display) -> ApiError {
    error!("db pool: {e}");
    ApiError::Rejection(
        StatusCode::SERVICE_UNAVAILABLE,
        "service unavailable".to_string(),
    )
}

pub fn db_error(e: tokio_postgres::Error) -> ApiError {
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
        None => error!(error = ?e, source = ?std::error::Error::source(&e), "db error"),
    }

    unavailable(e)
}
