use opentelemetry::trace::TracerProvider;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::runtime::Tokio;
use std::time::Duration;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_telemetry() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .unwrap_or_else(|_| "http://localhost:4317".to_string());

    let trace_exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(&endpoint)
        .with_timeout(Duration::from_secs(10));

    let tracer_provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(trace_exporter)
        .install_batch(Tokio)?;

    let tracer = tracer_provider.tracer("vidcall-backend");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_env("LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    Ok(())
}

pub fn shutdown_telemetry() {
    opentelemetry::global::shutdown_tracer_provider();
}
