use async_trait::async_trait;
use chrono::Utc;
use deadpool_postgres::Object;
use futures_util::TryFutureExt;
use tokio_postgres::Row;
use tokio_postgres::types::ToSql;
use tracing::Instrument;

pub mod conn;
mod constant;
pub mod error;
pub mod refresh_tokens;
pub mod room;
pub mod user;

pub use error::ModelError;

pub trait FromRow: Sized {
    fn from_row(row: &Row) -> Result<Self, ModelError>;
}

pub async fn fetch_opt<T: FromRow>(
    db: &Object,
    sql: &'static str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Option<T>, ModelError> {
    let stmt = db
        .prepare_cached(sql)
        .instrument(tracing::info_span!(
            "db.stmt",
            db.system = "postgresql",
            db.operation = "select"
        ))
        .await
        .map_err(ModelError::from)?;

    let row = db
        .query_opt(&stmt, params)
        .map_err(ModelError::from)
        .instrument(tracing::info_span!(
            "db.query",
            db.system = "postgresql",
            db.sql = sql
        ))
        .await?;

    row.as_ref().map(T::from_row).transpose()
}

pub async fn fetch_one<T: FromRow>(
    db: &Object,
    sql: &'static str,
    params: &[&(dyn ToSql + Sync)],
    not_found: &'static str,
) -> Result<T, ModelError> {
    fetch_opt(db, sql, params)
        .await?
        .ok_or_else(|| ModelError::not_found(not_found))
}

pub async fn exec(
    db: &Object,
    sql: &'static str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<(), ModelError> {
    let stmt = db
        .prepare_cached(sql)
        .instrument(tracing::info_span!("db.stmt", db.system = "postgresql"))
        .await
        .map_err(ModelError::from)?;
    db.execute(&stmt, params).await.map_err(ModelError::from)?;
    Ok(())
}

#[async_trait]
pub trait StandardModel {
    type Id;
    async fn create(&self, db: &Object) -> Result<Self::Id, ModelError>;
}
