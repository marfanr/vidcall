use std::fmt::Display;

use axum::{
    Json,
    body::{Body, Bytes},
    extract::FromRequest,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use tracing::error;
use validator::{Validate, ValidationErrors};

pub mod server;
pub mod v1;

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

#[doc = "simple used but not zero copy"]
pub struct ValidateJson<T>(T);

impl<S, T> FromRequest<S> for ValidateJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = ApiError;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        let b = Bytes::from_request(req, state)
            .await
            .map_err(|r| ApiError::Rejection(StatusCode::BAD_REQUEST, r.to_string()))?;

        let v = sonic_rs::from_slice::<T>(&b)
            .map_err(|r| ApiError::Rejection(StatusCode::BAD_REQUEST, r.to_string()))?;

        v.validate().map_err(ApiError::Validation)?;
        Ok(Self(v))
    }
}

pub struct FastJson<T>(pub T);

impl<T: sonic_rs::Serialize> IntoResponse for FastJson<T> {
    fn into_response(self) -> Response<Body> {
        match sonic_rs::to_vec(&self.0) {
            Ok(body) => Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "failed json").into_response(),
        }
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
        // bukan error dari server (koneksi putus, serialisasi parameter, dll.)
        None => error!(error = ?e, source = ?std::error::Error::source(&e), "db error"),
    }

    unavailable(e)
}
