use crate::api::{ApiError, db_error};
use crate::models::StandardModel;
use async_trait::async_trait;
use deadpool_postgres::{GenericClient, Object};
use futures_util::TryFutureExt;
use http::StatusCode;
use sonic_rs::{Deserialize, JsonValueTrait, Serialize, Value};
use tokio_postgres::error::SqlState;
use tokio_postgres::types::Json;
use tracing::Instrument;
use uuid::Uuid;
use crate::models::constant::CREATE_ROOM_SQL;

#[derive(Debug, Deserialize, Serialize)]
pub struct RoomOption {
    pub max_participants: Option<u32>,
}

// TODO: jadikan standar model seperti ini
#[derive(Debug)]
pub struct Room {
    pub alias: String,
    pub owner_id: Uuid,
    pub passcode: String,
    pub title: Option<String>,
    pub option: Option<Value>,
}

impl Room {
    pub fn new(alias: &'_ str, owner_id: Uuid, passcode: &'_ str) -> Self {
        Self {
            alias: alias.to_owned(),
            owner_id,
            passcode: passcode.to_owned(),
            title: None,
            option: None,
        }
    }
}

/// Standard Model
/// standard method used for every db model
#[async_trait]
impl StandardModel for Room {
    type Id = Uuid;
    #[tracing::instrument(skip_all)]
    async fn create(&self, db: Object) -> Result<Self::Id, ApiError> {
        let stmt = db
            .prepare_cached(
                CREATE_ROOM_SQL,
            )
            .instrument(tracing::info_span!(
                "db.stmt",
                db.system = "postgresql",
                db.operation = "insert"
            ))
            .map_err(db_error)
            .await?;

        let option = self
            .option
            .as_ref()
            .map(|v| sonic_rs::to_value(v).map(tokio_postgres::types::Json))
            .transpose()
            .map_err(|err| {
                ApiError::Rejection(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
            })?;

        let row = db
            .query_one(
                &stmt,
                &[
                    &self.owner_id,
                    &self.alias,
                    &self.passcode,
                    &self.title,
                    &option,
                ],
            )
            .instrument(tracing::info_span!("db.query", db.system = "postgresql"))
            .await
            .map_err(|e| {
                if let Some(dbe) = e.as_db_error() {
                    if dbe.code() == &SqlState::UNIQUE_VIOLATION {
                        return ApiError::Rejection(StatusCode::CONFLICT, "already exists".into());
                    }

                    if dbe.code() == &SqlState::FOREIGN_KEY_VIOLATION {
                        return ApiError::Rejection(
                            StatusCode::UNPROCESSABLE_ENTITY,
                            "user id does not exist".into(),
                        );
                    }
                }

                db_error(e)
            })?;

        Ok(row.get("id"))
    }
}
