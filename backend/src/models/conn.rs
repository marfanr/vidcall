use std::{env, time::Duration};

use deadpool_postgres::{
    ClientWrapper, Config, Hook, HookError, ManagerConfig, Pool, PoolConfig, RecyclingMethod,
    Runtime, SslMode,
};
use futures_util::future::join_all;
use rustls::{
    ClientConfig, RootCertStore,
    pki_types::{CertificateDer, pem::PemObject},
};
use tokio_postgres_rustls::MakeRustlsConnect;

use crate::models::constant::*;

pub async fn prewarm(pool: &Pool, n: usize) {
    let n = n.min(pool.status().max_size);

    let conns = join_all((0..n).map(|_| pool.get())).await;

    let results = join_all(conns.into_iter().map(|c| async move {
        let conn = c.map_err(|e| e.to_string())?;
        warm_statements(&conn).await.map_err(|e| e.to_string())
    }))
    .await;

    let ok = results.iter().filter(|r| r.is_ok()).count();
    for e in results.iter().filter_map(|r| r.as_ref().err()) {
        tracing::warn!(error = %e, "prewarm failed");
    }
    tracing::info!(ok, requested = n, "pool prewarmed");
}

#[tracing::instrument()]
pub fn init_db() -> Result<Pool, Box<dyn std::error::Error + Send + Sync>> {
    let password = env::var("POSTGRES_PASSWORD")?;
    let ca_path = env::var("PG_CA_CERT").unwrap_or_else(|_| "certs/ca.crt".into());
    let port: u16 = env::var("POSTGRES_PORT")
        .ok()
        .map(|p| p.parse())
        .transpose()?
        .unwrap_or(5432);

    let host = env::var("POSTGRES_HOST").unwrap_or_else(|_| "localhost".into());
    let dbname = env::var("POSTGRES_DB").unwrap_or_else(|_| "vidcall".into());
    let user = env::var("POSTGRES_USER").unwrap_or_else(|_| "postgres".into());
    let pool_size: usize = env::var("POSTGRES_POOL_SIZE")
        .ok()
        .map(|p| p.parse())
        .transpose()?
        .unwrap_or(32);

    let mut cfg = Config::new();
    cfg.host = Some(host);
    cfg.port = Some(port);
    cfg.user = Some(user);
    cfg.password = Some(password);
    cfg.ssl_mode = Some(SslMode::Require);
    cfg.dbname = Some(dbname);
    cfg.connect_timeout = Some(Duration::from_secs(5));
    cfg.keepalives = Some(true);
    cfg.keepalives_idle = Some(Duration::from_secs(30));
    cfg.pool = Some(PoolConfig::new(pool_size));
    cfg.manager = Some(ManagerConfig {
        recycling_method: RecyclingMethod::Fast,
    });

    let mut root_store = RootCertStore::empty();
    for cert in CertificateDer::pem_file_iter(&ca_path)? {
        root_store.add(cert?)?;
    }

    let tls_config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let tls = MakeRustlsConnect::new(tls_config);

    let pool = cfg
        .builder(tls)?
        .runtime(Runtime::Tokio1)
        .post_create(Hook::async_fn(|client, _| {
            Box::pin(async move {
                warm_statements(client)
                    .await
                    .map_err(|e| HookError::message(e.to_string()))
            })
        }))
        .build()?;

    Ok(pool)
}

#[tracing::instrument(skip(c))]
pub async fn warm_statements(c: &ClientWrapper) -> Result<(), tokio_postgres::Error> {
    c.prepare_cached(FIND_AND_MARK_SQL).await?;
    c.prepare_cached(CHECK_REUSE_AND_REVOKE_FAM_SQL).await?;
    c.prepare_cached(GET_USER_BY_EMAIL_SQL).await?;
    c.prepare_cached(CREATE_ROOM_SQL).await?;
    c.prepare_cached(GET_ROOM_BY_ALIAS_SQL).await?;
    c.prepare_cached(GET_USER_BY_ID_SQL).await?;
    c.prepare_cached(CREATE_USER_SQL).await?;
    Ok(())
}
