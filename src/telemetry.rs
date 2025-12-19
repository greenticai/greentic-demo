use std::collections::HashMap;

use anyhow::{Context, Result};
use greentic_telemetry::{
    TelemetryConfig as OtelConfig,
    export::{Compression, ExportConfig, ExportMode, Sampling},
    init_telemetry_from_config,
};
use serde::Deserialize;

use crate::config::{TelemetryConfig, TelemetrySource};

pub struct TelemetryHandle;

#[derive(Debug, Deserialize)]
struct TelemetryPayload {
    #[serde(default)]
    service_name: Option<String>,
    #[serde(default)]
    sampling: Option<SamplingPayload>,
    #[serde(default)]
    otlp: Option<OtlpPayload>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SamplingPayload {
    Ratio { ratio: f64 },
    Legacy(f64),
}

impl SamplingPayload {
    fn to_sampling(&self) -> Sampling {
        let ratio = match self {
            SamplingPayload::Ratio { ratio } | SamplingPayload::Legacy(ratio) => *ratio,
        };
        Sampling::TraceIdRatio(ratio)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
enum OtlpProtocol {
    Http,
    #[default]
    Grpc,
}

#[derive(Debug, Deserialize, Default)]
struct OtlpPayload {
    #[serde(default)]
    endpoint: Option<String>,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    protocol: Option<OtlpProtocol>,
    #[serde(default)]
    compression: Option<String>,
}

pub fn init(config: &TelemetryConfig) -> Result<Option<TelemetryHandle>> {
    match config {
        TelemetryConfig::Disabled => Ok(None),
        TelemetryConfig::Preconfigured { payload, source } => {
            let source_desc = match source {
                TelemetrySource::Env => "env var".to_string(),
                TelemetrySource::File(path) => format!("file {}", path.display()),
            };

            let settings: TelemetryPayload = serde_json::from_str(payload)
                .with_context(|| format!("invalid telemetry payload from {source_desc}"))?;

            let service_name = settings
                .service_name
                .clone()
                .unwrap_or_else(|| "greentic-demo".to_string());

            let export = build_export_config(&settings)?;

            init_telemetry_from_config(
                OtelConfig {
                    service_name: service_name.clone(),
                },
                export,
            )
            .with_context(|| "failed to initialize greentic telemetry pipeline")?;

            tracing::info!(
                service = %service_name,
                source = %source_desc,
                "telemetry pipeline initialized"
            );

            Ok(Some(TelemetryHandle))
        }
    }
}

fn build_export_config(settings: &TelemetryPayload) -> Result<ExportConfig> {
    let sampling = settings
        .sampling
        .as_ref()
        .map(SamplingPayload::to_sampling)
        .unwrap_or(Sampling::Parent);

    let Some(otlp) = settings.otlp.as_ref() else {
        let mut export = ExportConfig::json_default();
        export.sampling = sampling;
        return Ok(export);
    };

    let compression = otlp
        .compression
        .as_deref()
        .map(|c| c.to_ascii_lowercase())
        .and_then(|c| match c.as_str() {
            "gzip" => Some(Compression::Gzip),
            other => {
                tracing::warn!(compression = %other, "unsupported OTLP compression, ignoring");
                None
            }
        });

    Ok(ExportConfig {
        mode: match otlp.protocol.unwrap_or_default() {
            OtlpProtocol::Grpc => ExportMode::OtlpGrpc,
            OtlpProtocol::Http => ExportMode::OtlpHttp,
        },
        endpoint: otlp.endpoint.clone(),
        headers: otlp.headers.clone(),
        sampling,
        compression,
    })
}
