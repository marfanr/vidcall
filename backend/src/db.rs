use deadpool_postgres::Object;
use tokio_postgres::Row;
use tracing::instrument;

#[instrument(name = "db.query", skip_all, fields(db.system = "postgresql", db.operation = %operation))]
pub async fn traced_query(
    db: &Object,
    operation: &str,
) -> Result<Vec<Row>, deadpool_postgres::PoolError> {
    let start = std::time::Instant::now();
    let result = db.query(operation, &[]).await;
    let elapsed = start.elapsed();

    match &result {
        Ok(_) => {
            tracing::info!(db.duration_ms = elapsed.as_millis() as u64, "db query completed");
        }
        Err(e) => {
            tracing::error!(error = %e, "db query failed");
        }
    }

    result.map_err(Into::into)
}

#[instrument(name = "db.execute", skip_all, fields(db.system = "postgresql", db.operation = %operation))]
pub async fn traced_execute(
    db: &Object,
    operation: &str,
) -> Result<u64, deadpool_postgres::PoolError> {
    let start = std::time::Instant::now();
    let result = db.execute(operation, &[]).await;
    let elapsed = start.elapsed();

    match &result {
        Ok(count) => {
            tracing::info!(db.duration_ms = elapsed.as_millis() as u64, db.rows_affected = count, "db execute completed");
        }
        Err(e) => {
            tracing::error!(error = %e, "db execute failed");
        }
    }

    result.map_err(Into::into)
}

#[instrument(name = "db.query_one", skip_all, fields(db.system = "postgresql", db.operation = %operation))]
pub async fn traced_query_one(
    db: &Object,
    operation: &str,
) -> Result<Option<Row>, deadpool_postgres::PoolError> {
    let start = std::time::Instant::now();
    let result = db.query_opt(operation, &[]).await;
    let elapsed = start.elapsed();

    match &result {
        Ok(Some(_)) => {
            tracing::info!(db.duration_ms = elapsed.as_millis() as u64, "db query_one found");
        }
        Ok(None) => {
            tracing::info!(db.duration_ms = elapsed.as_millis() as u64, "db query_one not found");
        }
        Err(e) => {
            tracing::error!(error = %e, "db query_one failed");
        }
    }

    result.map_err(Into::into)
}
