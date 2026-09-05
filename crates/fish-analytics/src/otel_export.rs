use std::env;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::otel::OtelTracer;

/// Environment variables defined by the OpenTelemetry specification.
pub const ENV_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_ENDPOINT";
pub const ENV_TIMEOUT_MS: &str = "OTEL_EXPORTER_OTLP_TIMEOUT_MS";

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const TRACES_PATH: &str = "v1/traces";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OtelExportConfig {
    /// Full OTLP/HTTP traces endpoint (e.g. `http://localhost:4318/v1/traces`).
    pub endpoint: String,
    pub timeout: Duration,
}

impl OtelExportConfig {
    /// Read the standard `OTEL_EXPORTER_OTLP_*` environment variables.
    ///
    /// Returns `Ok(None)` when no endpoint is configured so callers can skip
    /// export silently; a malformed endpoint or timeout is an error.
    pub fn from_env() -> Result<Option<Self>, String> {
        Self::from_env_with(|key| env::var(key).ok())
    }

    /// Testable variant taking a variable lookup closure.
    pub fn from_env_with(lookup: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, String> {
        let Some(raw_endpoint) = lookup(ENV_ENDPOINT).filter(|v| !v.trim().is_empty()) else {
            return Ok(None);
        };
        let endpoint = normalize_endpoint(raw_endpoint.trim())?;

        let timeout = match lookup(ENV_TIMEOUT_MS) {
            Some(raw) if !raw.trim().is_empty() => {
                let ms: u64 = raw
                    .trim()
                    .parse()
                    .map_err(|_| format!("{ENV_TIMEOUT_MS} must be a positive integer"))?;
                Duration::from_millis(ms)
            }
            _ => DEFAULT_TIMEOUT,
        };

        Ok(Some(Self { endpoint, timeout }))
    }
}

/// Accept either the collector base URL (`http://host:4318`) or a path that
/// already ends in the traces route.
fn normalize_endpoint(raw: &str) -> Result<String, String> {
    if raw.ends_with(TRACES_PATH) {
        return Ok(raw.to_string());
    }
    let base = raw.trim_end_matches('/');
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err(format!(
            "{ENV_ENDPOINT} must be an http(s) URL, got `{raw}`"
        ));
    }
    Ok(format!("{base}/{TRACES_PATH}"))
}

#[derive(Debug)]
pub enum OtelExportError {
    Request(String),
    HttpStatus(u16),
}

impl std::fmt::Display for OtelExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OtelExportError::Request(msg) => write!(f, "OTLP export request failed: {msg}"),
            OtelExportError::HttpStatus(code) => {
                write!(f, "OTLP collector rejected the export with HTTP {code}")
            }
        }
    }
}

/// Exports recorded spans to an OTLP/HTTP + JSON collector endpoint using the
/// payload produced by [`OtelTracer::to_otlp_json`].
pub struct OtlpExporter {
    client: reqwest::Client,
    config: OtelExportConfig,
}

impl OtlpExporter {
    pub fn new(config: OtelExportConfig) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(config.timeout)
            .build()
            .map_err(|e| format!("failed to build OTLP HTTP client: {e}"))?;
        Ok(Self { client, config })
    }

    pub fn config(&self) -> &OtelExportConfig {
        &self.config
    }

    /// Push every recorded span to the collector. Returns the number of
    /// spans delivered; failures are errors, never silent drops.
    pub async fn export_tracer(&self, tracer: &OtelTracer) -> Result<usize, OtelExportError> {
        self.export_json(tracer.to_otlp_json(), tracer.span_count())
            .await
    }

    /// Same as [`Self::export_tracer`] but clears the tracer on success.
    pub async fn export_and_clear(&self, tracer: &OtelTracer) -> Result<usize, OtelExportError> {
        let count = self.export_tracer(tracer).await?;
        tracer.clear();
        Ok(count)
    }

    async fn export_json(
        &self,
        payload: serde_json::Value,
        expected_spans: usize,
    ) -> Result<usize, OtelExportError> {
        let response = self
            .client
            .post(&self.config.endpoint)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| OtelExportError::Request(e.to_string()))?;

        let status = response.status();
        if !status.is_success() {
            return Err(OtelExportError::HttpStatus(status.as_u16()));
        }
        Ok(expected_spans)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn config_with(env: &[(&str, &str)]) -> Result<Option<OtelExportConfig>, String> {
        let map: HashMap<String, String> = env
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect();
        OtelExportConfig::from_env_with(|key| map.get(key).cloned())
    }

    #[test]
    fn test_config_absent_endpoint_is_none() {
        assert_eq!(config_with(&[]).unwrap(), None);
        assert_eq!(config_with(&[(ENV_ENDPOINT, "   ")]).unwrap(), None);
    }

    #[test]
    fn test_config_base_url_appends_traces_path() {
        let cfg = config_with(&[(ENV_ENDPOINT, "http://collector:4318/")])
            .unwrap()
            .unwrap();
        assert_eq!(cfg.endpoint, "http://collector:4318/v1/traces");
        assert_eq!(cfg.timeout, DEFAULT_TIMEOUT);
    }

    #[test]
    fn test_config_full_path_and_timeout() {
        let cfg = config_with(&[
            (ENV_ENDPOINT, "https://otel.internal/v1/traces"),
            (ENV_TIMEOUT_MS, "2500"),
        ])
        .unwrap()
        .unwrap();
        assert_eq!(cfg.endpoint, "https://otel.internal/v1/traces");
        assert_eq!(cfg.timeout, Duration::from_millis(2500));
    }

    #[test]
    fn test_config_rejects_non_http_and_bad_timeout() {
        let err = config_with(&[(ENV_ENDPOINT, "grpc://collector")]).unwrap_err();
        assert!(err.contains("http(s)"));
        let err = config_with(&[(ENV_ENDPOINT, "http://collector"), (ENV_TIMEOUT_MS, "soon")])
            .unwrap_err();
        assert!(err.contains("positive integer"));
    }

    #[tokio::test]
    async fn test_export_posts_otlp_payload_to_mock_collector() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let collector = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 16 * 1024];
            let read = socket.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..read]).to_string();
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n")
                .await
                .unwrap();
            request
        });

        let tracer = OtelTracer::new("fish-build");
        tracer.record_span(tracer.start_span("compile_core").finish(true, None));

        let cfg = OtelExportConfig {
            endpoint: format!("http://{addr}/v1/traces"),
            timeout: Duration::from_secs(5),
        };
        let exporter = OtlpExporter::new(cfg).unwrap();
        let exported = exporter.export_tracer(&tracer).await.unwrap();
        assert_eq!(exported, 1);

        let request = collector.await.unwrap();
        assert!(
            request.starts_with("POST /v1/traces HTTP/1.1"),
            "got: {request}"
        );
        assert!(
            request
                .to_lowercase()
                .contains("content-type: application/json")
        );
        assert!(request.contains("resourceSpans"));
        assert!(request.contains("fish-build"));
    }

    #[tokio::test]
    async fn test_export_surfaces_collector_errors() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let collector = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 1024];
            let _ = socket.read(&mut buf).await;
            socket
                .write_all(b"HTTP/1.1 503 Service Unavailable\r\ncontent-length: 0\r\n\r\n")
                .await
                .unwrap();
        });

        let cfg = OtelExportConfig {
            endpoint: format!("http://{addr}/v1/traces"),
            timeout: Duration::from_secs(5),
        };
        let exporter = OtlpExporter::new(cfg).unwrap();
        let tracer = OtelTracer::new("fish-build");
        tracer.record_span(tracer.start_span("compile_core").finish(true, None));
        assert_eq!(tracer.span_count(), 1);

        match exporter.export_and_clear(&tracer).await {
            Err(OtelExportError::HttpStatus(503)) => {}
            other => panic!("expected HTTP 503 error, got {other:?}"),
        }
        collector.await.unwrap();

        assert_eq!(tracer.span_count(), 1);
    }

    #[tokio::test]
    async fn test_export_and_clear_resets_tracer_on_success() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let collector = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 8192];
            let _ = socket.read(&mut buf).await;
            socket
                .write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\n\r\n")
                .await
                .unwrap();
        });

        let tracer = OtelTracer::new("fish-build");
        tracer.record_span(tracer.start_span("link").finish(true, None));

        let cfg = OtelExportConfig {
            endpoint: format!("http://{addr}/v1/traces"),
            timeout: Duration::from_secs(5),
        };
        let exporter = OtlpExporter::new(cfg).unwrap();
        assert_eq!(exporter.export_and_clear(&tracer).await.unwrap(), 1);
        assert_eq!(tracer.span_count(), 0);
        collector.await.unwrap();
    }
}
