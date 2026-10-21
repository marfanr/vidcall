use crate::models::ModelError;
use crate::models::constant::{CREATE_ROOM_SQL, GET_ROOM_BY_ALIAS_SQL, GET_ROOM_BY_ID_SQL};
use crate::models::{FromRow, StandardModel, fetch_one};
use async_trait::async_trait;
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
    pub owner: Option<Arc<str>>,
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
            owner: None,
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
    pub async fn get_by_alias(db: &Object, alias: &str) -> Result<Self, ModelError> {
        fetch_one(db, GET_ROOM_BY_ALIAS_SQL, &[&alias], "room not found").await
    }

    /// Typed view over the options jsonb payload.
    pub fn options_as<T: DeserializeOwned>(&self) -> Result<Option<T>, ModelError> {
        let Some(options) = &self.options else {
            return Ok(None);
        };

        T::deserialize(options).map(Some).map_err(|e| {
            tracing::error!("invalid room options: {e}");
            ModelError::unprocessable("invalid room options")
        })
    }

    pub async fn get(db: &Object, id: Uuid) -> Result<Self, ModelError> {
        fetch_one(db, GET_ROOM_BY_ID_SQL, &[&id], "room not found").await
    }
}

#[async_trait]
impl StandardModel for Room {
    type Id = Uuid;

    #[tracing::instrument(skip_all)]
    async fn create(&self, db: &Object) -> Result<Self::Id, ModelError> {
        let stmt = db
            .prepare_cached(CREATE_ROOM_SQL)
            .await
            .map_err(ModelError::from)?;

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

        row.try_get("id").map_err(ModelError::from)
    }
}

/// Map a unique/foreign key violation on room to a model error.
fn room_write_error(e: tokio_postgres::Error) -> ModelError {
    let Some(db_err) = e.as_db_error() else {
        return ModelError::Database(e);
    };

    match *db_err.code() {
        SqlState::UNIQUE_VIOLATION => ModelError::conflict("alias already exists"),
        SqlState::FOREIGN_KEY_VIOLATION => {
            ModelError::unprocessable("user id does not exist")
        }
        _ => ModelError::Database(e),
    }
}

impl FromRow for Room {
    fn from_row(r: &Row) -> Result<Self, ModelError> {
        let id: Uuid = r.try_get("id").map_err(ModelError::from)?;
        let alias: String = r.try_get("alias").map_err(ModelError::from)?;
        let owner_id: Uuid = r.try_get("owner_id").map_err(ModelError::from)?;
        let passcode: Option<String> = r.try_get("passcode").map_err(ModelError::from)?;
        let title: Option<String> = r.try_get("title").map_err(ModelError::from)?;
        let options: Option<Value> = r.try_get("options").map_err(ModelError::from)?;
        let owner: Option<String> = r.try_get("owner").map_err(ModelError::from)?;

        Ok(Self {
            id: Some(id),
            alias: Arc::from(alias),
            owner_id,
            owner: owner.map(Arc::from),
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
