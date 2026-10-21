use axum::{
    body::{Body, Bytes},
    extract::FromRequest,
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use serde::de::DeserializeOwned;
use validator::Validate;

pub mod error;
pub mod metrics;
pub mod server;
mod v1;

pub use error::{ApiError, WsError, poll_error};

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
