use axum::{
    Json, extract::{FromRequest, rejection::JsonRejection}, http::{StatusCode, status}, response::IntoResponse,
};
use serde::{de::DeserializeOwned};
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
                Json(serde_json::json!({
                    "error": "internal_error"
                })),
            )
                .into_response(),

            Self::Rejection(status, msg) => (
                status,
                Json(serde_json::json!({
                    "error": msg
                })),
            )
                .into_response(),

            Self::Validation(val) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({
                    "error": "validation fields",
                    "fields": val,
                })),
            )
                .into_response(),
        }
    }
}

pub struct ValidateJson<T>(T);

impl <S, T> FromRequest<S> for ValidateJson<T> 
where
    S: Send + Sync,
    T: DeserializeOwned + Validate
{
    type Rejection = ApiError;

    async fn from_request(
        req: axum::extract::Request,
        state: &S,
    ) -> Result<Self, Self::Rejection>
    {
        let Json(v) = Json::<T>::from_request(req, state)
            .await
            .map_err(
                |r: JsonRejection| ApiError::Rejection(r.status(), r.body_text())
            )?;
        v.validate().map_err(ApiError::Validation)?;
        Ok(Self(v))
    }
}
