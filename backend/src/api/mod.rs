use axum::{Json, http::StatusCode, response::IntoResponse};

pub mod server;
pub mod room;

// Error response
#[derive(Debug)]
pub enum ApiError {
    NotFound,
    Rejection(StatusCode, String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({
                "error": "internal_error"
            }))
        )
        .into_response()
    }
}
