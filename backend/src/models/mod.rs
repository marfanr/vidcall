use async_trait::async_trait;
use deadpool_postgres::Object;
use uuid::Uuid;
use crate::api::ApiError;

pub mod conn;
pub mod refresh_tokens;
pub mod room;
pub mod user;
mod constant;

#[async_trait]
pub trait StandardModel {
    type Id;
    async fn create(&self, db: Object) -> Result<Self::Id, ApiError>;
}
