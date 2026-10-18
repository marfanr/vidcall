use std::{env, time::Duration};

use deadpool_postgres::{Config, ManagerConfig, Pool, Runtime, SslMode};
use rustls::{
    ClientConfig, RootCertStore,
    pki_types::{CertificateDer, pem::PemObject},
};
use tokio_postgres_rustls::MakeRustlsConnect;

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
    cfg.pool = Some(deadpool_postgres::PoolConfig::new(pool_size));
    cfg.manager = Some(ManagerConfig {
        recycling_method: deadpool_postgres::RecyclingMethod::Verified,
    });

    let mut root_store = RootCertStore::empty();
    for cert in CertificateDer::pem_file_iter(&ca_path)? {
        root_store.add(cert?)?;
    }

    let tls_config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let tls = MakeRustlsConnect::new(tls_config);

    Ok(cfg.create_pool(Some(Runtime::Tokio1), tls)?)
}
