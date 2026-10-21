use crate::api::{ApiError, db_error};
use async_trait::async_trait;
use deadpool_postgres::Object;
use futures_util::TryFutureExt;
use http::StatusCode;
use tokio_postgres::Row;
use tokio_postgres::types::ToSql;
use tracing::Instrument;

pub mod conn;
mod constant;
pub mod refresh_tokens;
pub mod room;
pub mod user;

pub trait FromRow: Sized {
    fn from_row(row: &Row) -> Result<Self, ApiError>;
}

pub async fn fetch_opt<T: FromRow>(
    db: &Object,
    sql: &'static str,
    params: &[&(dyn ToSql + Sync)],
) -> Result<Option<T>, ApiError> {
    let stmt = db
        .prepare_cached(sql)
        .instrument(tracing::info_span!(
            "db.stmt",
            db.system = "postgresql",
            db.operation = "select"
        ))
        .await
        .map_err(db_error)?;

    let row = db
        .query_opt(&stmt, params)
        .map_err(db_error)
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
) -> Result<T, ApiError> {
    fetch_opt(db, sql, params)
        .await?
        .ok_or_else(|| ApiError::Rejection(StatusCode::NOT_FOUND, not_found.to_owned()))
}

#[async_trait]
pub trait StandardModel {
    type Id;
    async fn create(&self, db: &Object) -> Result<Self::Id, ApiError>;
}
