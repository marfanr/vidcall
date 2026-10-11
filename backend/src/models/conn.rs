use std::{env, sync::Arc, time::Duration};

use rustls::{
    pki_types::{pem::PemObject, CertificateDer},
    ClientConfig, RootCertStore,
};
use tokio_postgres::{config::SslMode, Client, Config};
use tokio_postgres_rustls::MakeRustlsConnect;

pub async fn init_db() -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
    let password = env::var("POSTGRES_PASSWORD")?;
    let ca_path = env::var("PG_CA_CERT").unwrap_or_else(|_| "certs/ca.crt".into());

    let mut cfg = Config::new();
    cfg.host("localhost")
        .port(5432)
        .user("postgres")
        .password(password)
        .ssl_mode(SslMode::Require)
        .connect_timeout(Duration::from_secs(5));

    let mut root_store = RootCertStore::empty();
    for cert in CertificateDer::pem_file_iter(&ca_path)? {
        root_store.add(cert?)?;
    }

    let tls_config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let (client, connection) = cfg.connect(MakeRustlsConnect::new(tls_config)).await?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("PostgreSQL connection error: {e}");
        }
    });

    Ok(client)
}