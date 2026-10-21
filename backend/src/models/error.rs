use std::error::Error;
use std::fmt;

/// Errors produced by the model layer.
///
/// Models return this type instead of [`crate::api::ApiError`]; the API layer
/// converts it via `From<ModelError> for ApiError`.
#[derive(Debug)]
pub enum ModelError {
    NotFound(String),
    Conflict(String),
    Unprocessable(String),
    Database(tokio_postgres::Error),
    Internal(String),
}

impl ModelError {
    #[must_use]
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    #[must_use]
    pub fn conflict(msg: impl Into<String>) -> Self {
        Self::Conflict(msg.into())
    }

    #[must_use]
    pub fn unprocessable(msg: impl Into<String>) -> Self {
        Self::Unprocessable(msg.into())
    }

    #[must_use]
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }
}

impl fmt::Display for ModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(msg) => write!(f, "not found: {msg}"),
            Self::Conflict(msg) => write!(f, "conflict: {msg}"),
            Self::Unprocessable(msg) => write!(f, "unprocessable: {msg}"),
            Self::Database(e) => write!(f, "database error: {e}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl Error for ModelError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Database(e) => Some(e),
            _ => None,
        }
    }
}

impl From<tokio_postgres::Error> for ModelError {
    fn from(e: tokio_postgres::Error) -> Self {
        Self::Database(e)
    }
}
