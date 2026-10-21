use crate::api::{ApiError, db_error};
use crate::models::constant::{CREATE_ROOM_SQL, GET_ROOM_BY_ALIAS_SQL};
use crate::models::{FromRow, StandardModel, fetch_one};
use async_trait::async_trait;
use axum::http::StatusCode;
use deadpool_postgres::Object;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio_postgres::Row;
use tokio_postgres::error::SqlState;
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct RoomOption {
    pub max_participants: Option<u32>,
}

#[derive(Debug)]
pub struct Room {
    pub id: Option<Uuid>,
    pub alias: Arc<str>,
    pub owner_id: Uuid,
    pub passcode: Option<Arc<str>>,
    pub title: Option<Arc<str>>,
    pub options: Option<Value>,
}

impl Room {
    pub fn new(alias: &str, owner_id: Uuid, passcode: Option<&str>) -> Self {
        Self {
            id: None,
            alias: Arc::from(alias),
            owner_id,
            passcode: passcode.map(Arc::from),
            title: None,
            options: None,
        }
    }

    /// Attach the options jsonb payload of the room.
    pub fn with_options(mut self, options: Value) -> Self {
        self.options = Some(options);
        self
    }

    #[tracing::instrument(skip_all)]
    pub async fn get_by_alias(db: &Object, alias: &str) -> Result<Self, ApiError> {
        fetch_one(db, GET_ROOM_BY_ALIAS_SQL, &[&alias], "room not found").await
    }

    /// Typed view over the options jsonb payload.
    pub fn options_as<T: DeserializeOwned>(&self) -> Result<Option<T>, ApiError> {
        let Some(options) = &self.options else {
            return Ok(None);
        };

        T::deserialize(options).map(Some).map_err(|e| {
            tracing::error!("invalid room options: {e}");
            ApiError::Rejection(
                StatusCode::UNPROCESSABLE_ENTITY,
                "invalid room options".into(),
            )
        })
    }
}

#[async_trait]
impl StandardModel for Room {
    type Id = Uuid;

    #[tracing::instrument(skip_all)]
    async fn create(&self, db: &Object) -> Result<Self::Id, ApiError> {
        let stmt = db.prepare_cached(CREATE_ROOM_SQL).await.map_err(db_error)?;

        let alias: &str = &self.alias;
        let passcode: Option<&str> = self.passcode.as_deref();
        let title: Option<&str> = self.title.as_deref();
        let options: Option<&Value> = self.options.as_ref();

        let row = db
            .query_one(
                &stmt,
                &[&self.owner_id, &alias, &passcode, &title, &options],
            )
            .await
            .map_err(room_write_error)?;

        row.try_get("id").map_err(db_error)
    }
}

/// Map a unique/foreign key violation on room to a rejection.
fn room_write_error(e: tokio_postgres::Error) -> ApiError {
    let Some(db_err) = e.as_db_error() else {
        return db_error(e);
    };

    match *db_err.code() {
        SqlState::UNIQUE_VIOLATION => {
            ApiError::Rejection(StatusCode::CONFLICT, "alias already exists".into())
        }
        SqlState::FOREIGN_KEY_VIOLATION => ApiError::Rejection(
            StatusCode::UNPROCESSABLE_ENTITY,
            "user id does not exist".into(),
        ),
        _ => db_error(e),
    }
}

impl FromRow for Room {
    fn from_row(r: &Row) -> Result<Self, ApiError> {
        let id: Uuid = r.try_get("id").map_err(db_error)?;
        let alias: String = r.try_get("alias").map_err(db_error)?;
        let owner_id: Uuid = r.try_get("owner_id").map_err(db_error)?;
        let passcode: Option<String> = r.try_get("passcode").map_err(db_error)?;
        let title: Option<String> = r.try_get("title").map_err(db_error)?;
        let options: Option<Value> = r.try_get("options").map_err(db_error)?;

        Ok(Self {
            id: Some(id),
            alias: Arc::from(alias),
            owner_id,
            passcode: passcode.map(Arc::from),
            title: title.map(Arc::from),
            options,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_room_has_no_id_and_no_options() {
        let room = Room::new("demo", Uuid::now_v7(), None);

        assert!(room.id.is_none());
        assert!(room.options.is_none());
        assert!(room.passcode.is_none());
        assert_eq!(room.alias.as_ref(), "demo");
    }

    #[test]
    fn options_as_reads_typed_payload() {
        let room = Room::new("demo", Uuid::now_v7(), None)
            .with_options(serde_json::json!({ "max_participants": 10 }));

        let options: RoomOption = room.options_as().unwrap().unwrap();

        assert_eq!(options.max_participants, Some(10));
    }

    #[test]
    fn options_as_returns_none_without_payload() {
        let room = Room::new("demo", Uuid::now_v7(), None);

        assert!(room.options_as::<RoomOption>().unwrap().is_none());
    }

    #[test]
    fn options_as_reports_invalid_payload() {
        let room = Room::new("demo", Uuid::now_v7(), None)
            .with_options(serde_json::json!("not-an-object"));

        assert!(room.options_as::<RoomOption>().is_err());
    }
}
