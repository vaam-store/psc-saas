use opentelemetry::trace::TraceError;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, trace as sdktrace};
use tonic::metadata::*;
use tracing::Subscriber;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, Registry};

pub fn init_subscriber(service_name: &str) -> impl Subscriber {
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint("http://jaeger:4317")
        .with_metadata(MetadataMap::from_headers(Default::default()));

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", service_name.to_string()),
        ])))
        .install_batch(opentelemetry_sdk::runtime::Tokio)
        .unwrap();

    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(telemetry_layer)
}

pub fn init_tracer(service_name: &str) -> Result<sdktrace::Tracer, TraceError> {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://jaeger:4317"),
        )
        .with_trace_config(sdktrace::config().with_resource(Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", service_name.to_string()),
        ])))
        .install_batch(opentelemetry_sdk::runtime::Tokio)
}

pub fn setup_telemetry(service_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let tracer = init_tracer(service_name)?;
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let subscriber = Registry::default()
        .with(EnvFilter::from_default_env())
        .with(telemetry_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}
