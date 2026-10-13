use axum::{
    Json,
    body::Body,
    extract::{FromRequest, rejection::JsonRejection},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use validator::{Validate, ValidationErrors};

pub mod room;
pub mod server;

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

pub struct ValidateJson<T>(T);

impl<S, T> FromRequest<S> for ValidateJson<T>
where
    S: Send + Sync,
    T: DeserializeOwned + Validate,
{
    type Rejection = ApiError;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(v) = Json::<T>::from_request(req, state)
            .await
            .map_err(|r: JsonRejection| ApiError::Rejection(r.status(), r.body_text()))?;
        v.validate().map_err(ApiError::Validation)?;
        Ok(Self(v))
    }
}

pub struct FastJson<T>(pub T);

impl<T: sonic_rs::Serialize> IntoResponse for FastJson<T> {
    fn into_response(self) -> Response<Body> {
        match sonic_rs::to_string(&self.0) {
            Ok(body) => Response::builder()
                .status(StatusCode::OK)
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
            Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "failed json").into_response(),
        }
    }
}
