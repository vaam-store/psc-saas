use std::error::Error;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

use anyhow::Result;
use axum_otel_metrics::{HttpMetricsLayer, HttpMetricsLayerBuilder, PathSkipper};
use opentelemetry::global;
use opentelemetry_otlp::{Compression, Protocol, SpanExporter, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::metrics::SdkMeterProvider;
use opentelemetry_sdk::trace::{RandomIdGenerator, Sampler, TracerProviderBuilder};
use opentelemetry_sdk::Resource;
use std::time::Duration;
use tracing_subscriber::layer::SubscriberExt;

/// Minimal telemetry shim compatible with newer OpenTelemetry crates.
///
/// NOTE:
/// This is a conservative temporary implementation to keep the workspace
/// compiling while we upgrade the OpenTelemetry stack to 0.30+. The previous
/// implementation used older opentelemetry_otlp APIs that changed in the 0.30
/// series. A full port (with OTLP exporter and batch pipeline) should replace
/// this shim later.
///
/// Public surface kept the same:
/// - init_subscriber(service_name) -> impl Subscriber
/// - init_tracer(service_name) -> Result<..., _>
/// - setup_telemetry(service_name) -> Result<(), Box<dyn Error>>
pub fn init_subscriber(_service_name: &str) -> Result<(), Box<dyn Error>> {
    let subscriber = tracing_subscriber::registry().with(EnvFilter::from_default_env());
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

/// init_tracer currently returns an error indicating telemetry is not yet wired.
///
/// This intentionally avoids depending on unstable/private SDK internals.
/// Replace this with a real Tracer creation when porting to the newer APIs.
pub fn init_tracer_provider(service_name: &str) -> Result<(), Box<dyn Error>> {
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_compression(Compression::Gzip)
        .with_timeout(Duration::from_secs(3))
        .build()?;

    let resource = Resource::builder()
        .with_service_name(service_name.to_string())
        .build();

    let tracer_provider = TracerProviderBuilder::default()
        .with_batch_exporter(exporter)
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .with_max_events_per_span(16)
        .with_max_attributes_per_span(16)
        .with_resource(resource)
        .build();

    global::set_tracer_provider(tracer_provider.clone());

    Ok(())
}

pub fn init_meter_provider(service_name: &str) -> Result<()> {
    let prometheus_exporter = opentelemetry_prometheus::exporter()
        .with_registry(prometheus::default_registry().clone())
        .build()?;

    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_tonic()
        .with_compression(Compression::Gzip)
        .with_protocol(Protocol::Grpc)
        .with_timeout(Duration::from_secs(3))
        .build()?;

    let reader = opentelemetry_sdk::metrics::PeriodicReader::builder(exporter)
        .with_interval(Duration::from_secs(3))
        .build();

    let meter_provider = SdkMeterProvider::builder()
        .with_reader(prometheus_exporter)
        .with_reader(reader)
        .with_resource(
            Resource::builder()
                .with_service_name(service_name.to_string())
                .build(),
        )
        .build();

    global::set_meter_provider(meter_provider.clone());

    Ok(())
}

/// Set up global subscriber. For now we set a simple env-filter subscriber so
/// logs/traces are routed through tracing without an OpenTelemetry exporter.
/// Replace with an OTLP + tracing integration during a proper port.
pub fn setup_telemetry(service_name: &str) -> Result<(), Box<dyn Error>> {
    let _ = init_tracer_provider(service_name)?;
    let _ = init_meter_provider(service_name)?;
    let _ = init_subscriber(service_name)?;

    Ok(())
}

pub fn metric_layers(skip: Arc<dyn Fn(&str) -> bool + 'static + Send + Sync>) -> HttpMetricsLayer {
    let metrics = HttpMetricsLayerBuilder::default()
        .with_skipper(PathSkipper::new_with_fn(skip))
        .build();

    metrics
}
