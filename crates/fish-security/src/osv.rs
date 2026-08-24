//! Live advisory lookups against an OSV (Open Source Vulnerabilities)
//! service.
//!
//! The embedded rule table in `backend.rs` is a stale snapshot; this module
//! talks to a real OSV instance instead. The endpoint is fully configurable
//! so air-gapped installs can point at a mirror while the default targets
//! the public `api.osv.dev`.
//!
//! Protocol used (OSV API v1):
//!   POST `{base}/querybatch` — batched package queries, returns per-query
//!                              lists of vulnerability ids
//!   GET  `{base}/vulns/{id}` — full vulnerability document

use std::collections::HashMap;
use std::env;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::error::{SecurityError, SecurityResult};
use crate::vulnerability::{Severity, Vulnerability, VulnerabilitySource};

/// Environment variable holding the OSV base URL.
pub const ENV_OSV_ENDPOINT: &str = "FISH_OSV_ENDPOINT";
/// Environment variable holding the request timeout in milliseconds.
pub const ENV_OSV_TIMEOUT_MS: &str = "FISH_OSV_TIMEOUT_MS";

pub const DEFAULT_OSV_BASE: &str = "https://api.osv.dev/v1";
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(15);
/// Keep batches modest; OSV handles this size comfortably.
const MAX_QUERIES_PER_BATCH: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsvConfig {
    /// Base URL including the version segment, e.g. `https://api.osv.dev/v1`.
    pub base_url: String,
    pub timeout: Duration,
}

impl Default for OsvConfig {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_OSV_BASE.to_string(),
            timeout: DEFAULT_TIMEOUT,
        }
    }
}

impl OsvConfig {
    /// Build a config from environment variables. Returns `Ok(None)` when no
    /// endpoint is configured so callers fall back to the embedded database;
    /// a present-but-invalid configuration is an error.
    pub fn from_env_with(lookup: impl Fn(&str) -> Option<String>) -> Result<Option<Self>, String> {
        let Some(base) = lookup(ENV_OSV_ENDPOINT).filter(|v| !v.trim().is_empty()) else {
            return Ok(None);
        };
        let trimmed = base.trim().trim_end_matches('/').to_string();
        if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            return Err(format!(
                "{ENV_OSV_ENDPOINT} must be an http(s) URL, got `{trimmed}`"
            ));
        }
        let mut config = Self {
            base_url: trimmed,
            timeout: DEFAULT_TIMEOUT,
        };
        if let Some(raw) = lookup(ENV_OSV_TIMEOUT_MS).filter(|v| !v.trim().is_empty()) {
            let ms: u64 = raw
                .trim()
                .parse()
                .map_err(|_| format!("{ENV_OSV_TIMEOUT_MS} must be a positive integer"))?;
            config.timeout = Duration::from_millis(ms);
        }
        Ok(Some(config))
    }

    pub fn from_env() -> Result<Option<Self>, String> {
        Self::from_env_with(|key| env::var(key).ok())
    }
}

/// Client for one configured OSV service.
#[derive(Clone)]
pub struct OsvClient {
    http: reqwest::Client,
    config: OsvConfig,
}

impl OsvClient {
    pub fn new(config: OsvConfig) -> SecurityResult<Self> {
        let http = reqwest::Client::builder().timeout(config.timeout).build()?;
        Ok(Self { http, config })
    }

    /// Build from environment, or `None` when OSV is not configured.
    pub fn from_env() -> SecurityResult<Option<Self>> {
        match OsvConfig::from_env_with(|k| env::var(k).ok()) {
            Ok(Some(config)) => Ok(Some(Self::new(config)?)),
            Ok(None) => Ok(None),
            Err(message) => Err(SecurityError::ApiError(message)),
        }
    }

    pub fn config(&self) -> &OsvConfig {
        &self.config
    }

    /// Query vulnerabilities for every `(ecosystem, name, version)` triple.
    ///
    /// The returned vector aligns index-for-index with the input. Packages
    /// with no known advisories map to empty vectors; transport or API
    /// failures surface as errors instead of silently-empty results. Full
    /// vulnerability documents are fetched once per unique id and reused
    /// across packages.
    pub async fn query_packages(
        &self,
        packages: &[(String, String, String)],
    ) -> SecurityResult<Vec<Vec<Vulnerability>>> {
        let mut out: Vec<Vec<Vulnerability>> = vec![Vec::new(); packages.len()];
        let mut cache: HashMap<String, Vulnerability> = HashMap::new();

        for (base, chunk) in packages.chunks(MAX_QUERIES_PER_BATCH).enumerate() {
            let batch_base = base * MAX_QUERIES_PER_BATCH;
            let queries: Vec<serde_json::Value> = chunk
                .iter()
                .map(|(ecosystem, name, version)| {
                    serde_json::json!({
                        "package": { "name": name, "ecosystem": ecosystem },
                        "version": version,
                    })
                })
                .collect();

            let resp = self
                .http
                .post(format!("{}/querybatch", self.config.base_url))
                .json(&serde_json::json!({ "queries": queries }))
                .send()
                .await?;
            if !resp.status().is_success() {
                return Err(SecurityError::ApiError(format!(
                    "OSV querybatch returned HTTP {}",
                    resp.status()
                )));
            }
            let body: QueryBatchResponse = resp.json().await?;

            for (offset, result) in body.results.into_iter().enumerate() {
                for vuln_id in result.vuln_ids {
                    let mut vuln = match cache.get(&vuln_id) {
                        Some(existing) => existing.clone(),
                        None => {
                            let fetched = self.fetch_vuln(&vuln_id).await?;
                            cache.insert(vuln_id.clone(), fetched.clone());
                            fetched
                        }
                    };
                    // The OSV document is package-agnostic on the fetch path;
                    // stamp the queried package so callers get a complete
                    // record.
                    if let Some((_, name, _)) = chunk.get(offset) {
                        vuln.package = name.clone();
                    }
                    if let Some(slot) = out.get_mut(batch_base + offset) {
                        slot.push(vuln);
                    }
                }
            }
        }
        Ok(out)
    }

    async fn fetch_vuln(&self, id: &str) -> SecurityResult<Vulnerability> {
        let resp = self
            .http
            .get(format!("{}/vulns/{}", self.config.base_url, id))
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(SecurityError::ApiError(format!(
                "OSV fetch of `{id}` returned HTTP {}",
                resp.status()
            )));
        }
        let doc: OsvVuln = resp.json().await?;
        Ok(map_osv_vuln(doc))
    }
}

// ---------------------------------------------------------------------------
// Wire format
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct QueryBatchResponse {
    #[serde(default)]
    results: Vec<QueryBatchResult>,
}

#[derive(Debug, Deserialize)]
struct QueryBatchResult {
    #[serde(
        rename = "vulns",
        default,
        deserialize_with = "deserialize_id_only_vulns"
    )]
    vuln_ids: Vec<String>,
}

/// querybatch returns `vulns` as id-only stubs (`{"id": "OSV-..."}`);
/// accept both that shape and bare strings for tolerance.
fn deserialize_id_only_vulns<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[allow(dead_code)]
    #[derive(Deserialize)]
    struct IdOnly {
        id: String,
    }
    let raw: Vec<serde_json::Value> = Vec::deserialize(deserializer)?;
    Ok(raw
        .into_iter()
        .filter_map(|v| {
            if let Some(id) = v.get("id").and_then(|i| i.as_str()) {
                Some(id.to_string())
            } else {
                v.as_str().map(str::to_string)
            }
        })
        .collect())
}

#[derive(Debug, Deserialize)]
struct OsvVuln {
    id: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    details: Option<String>,
    #[serde(default)]
    published: Option<DateTime<Utc>>,
    #[serde(default)]
    modified: Option<DateTime<Utc>>,
    #[serde(default)]
    affected: Vec<OsvAffected>,
    #[serde(default)]
    references: Vec<OsvReference>,
    #[serde(default)]
    database_specific: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct OsvAffected {
    #[serde(default)]
    ranges: Vec<OsvRange>,
}

#[derive(Debug, Deserialize)]
struct OsvRange {
    #[serde(rename = "type", default)]
    range_type: String,
    #[serde(default)]
    events: Vec<std::collections::BTreeMap<String, String>>,
}

#[derive(Debug, Deserialize)]
struct OsvReference {
    url: String,
}

fn map_osv_vuln(doc: OsvVuln) -> Vulnerability {
    // Fixed version: first "fixed" event in any SEMVER/ECOSYSTEM range.
    let fixed_version = doc
        .affected
        .iter()
        .flat_map(|a| a.ranges.iter())
        .find_map(|r| {
            if matches!(r.range_type.as_str(), "SEMVER" | "ECOSYSTEM") {
                r.events.iter().find_map(|e| e.get("fixed").cloned())
            } else {
                None
            }
        });

    // GHSA-style human labels land in database_specific.severity; anything
    // else stays Unrated rather than guessed.
    let severity = doc
        .database_specific
        .get("severity")
        .and_then(|v| v.as_str())
        .map(|label| match label.to_ascii_uppercase().as_str() {
            "LOW" => Severity::Low,
            "MODERATE" | "MEDIUM" => Severity::Medium,
            "HIGH" => Severity::High,
            "CRITICAL" => Severity::Critical,
            _ => Severity::None,
        })
        .unwrap_or(Severity::None);

    let mut references: Vec<String> = doc.references.into_iter().map(|r| r.url).collect();
    references.sort();
    references.dedup();

    let mut vuln = Vulnerability::new(doc.id.clone(), String::new(), severity);
    vuln.source = VulnerabilitySource::OSV;
    vuln.description = doc.summary.or(doc.details).unwrap_or_default();
    vuln.published_date = doc.published;
    vuln.modified_date = doc.modified;
    vuln.references = references;
    if let Some(fixed) = fixed_version {
        vuln.fixed_version = Some(fixed);
    }
    vuln
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    const QUERYBATCH_BODY: &str =
        r#"{"results":[{"vulns":[{"id":"RUSTSEC-2024-0332"}]},{"vulns":[]}]}"#;
    const VULN_BODY: &str = r#"{
        "id": "RUSTSEC-2024-0332",
        "summary": "h2 CONTINUATION flood",
        "published": "2024-04-04T00:00:00Z",
        "affected": [{
            "ranges": [{"type": "SEMVER", "events": [{"fixed": "0.3.26"}, {"introduced": "0.0.0"}]}]
        }],
        "references": [
            {"type": "ADVISORY", "url": "https://rustsec.org/advisories/RUSTSEC-2024-0332"},
            {"type": "PACKAGE", "url": "https://crates.io/crates/h2"}
        ],
        "database_specific": {"severity": "HIGH"}
    }"#;

    /// Minimal HTTP/1.1 server answering one querybatch POST and any number
    /// of /vulns/{id} GETs with canned documents.
    async fn spawn_mock_collector() -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            for _ in 0..2 {
                let Ok((mut socket, _)) = listener.accept().await else {
                    return;
                };
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 16 * 1024];
                    let read = socket.read(&mut buf).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&buf[..read]).to_string();

                    if request.contains("querybatch") {
                        let body = format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                            QUERYBATCH_BODY.len(),
                            QUERYBATCH_BODY
                        );
                        let _ = socket.write_all(body.as_bytes()).await;
                    } else if request.contains("/vulns/") {
                        let body = format!(
                            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                            VULN_BODY.len(),
                            VULN_BODY
                        );
                        let _ = socket.write_all(body.as_bytes()).await;
                    } else {
                        let _ = socket
                            .write_all(b"HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n")
                            .await;
                    }
                });
            }
        });
        (format!("http://{addr}/v1"), handle)
    }

    #[tokio::test]
    async fn test_query_packages_maps_osv_documents() {
        let (base, server) = spawn_mock_collector().await;
        let client = OsvClient::new(OsvConfig {
            base_url: base,
            timeout: Duration::from_secs(5),
        })
        .unwrap();

        let packages = vec![
            (
                "crates.io".to_string(),
                "h2".to_string(),
                "0.3.24".to_string(),
            ),
            (
                "crates.io".to_string(),
                "serde".to_string(),
                "1.0.200".to_string(),
            ),
        ];
        let results = client.query_packages(&packages).await.unwrap();

        assert_eq!(results.len(), 2);
        // h2 has the advisory; serde is clean.
        assert_eq!(results[0].len(), 1);
        assert_eq!(results[1].len(), 0);

        let vuln = &results[0][0];
        assert_eq!(vuln.id, "RUSTSEC-2024-0332");
        assert_eq!(vuln.package, "h2");
        assert_eq!(vuln.source, VulnerabilitySource::OSV);
        assert_eq!(
            vuln.severity,
            Severity::High,
            "database_specific label maps"
        );
        assert_eq!(
            vuln.fixed_version.as_deref(),
            Some("0.3.26"),
            "first fixed event wins"
        );
        assert!(vuln.references.iter().any(|u| u.contains("rustsec.org")));
        assert!(vuln.published_date.is_some());

        server.await.unwrap();
    }

    #[test]
    fn test_config_from_env() {
        let cfg = OsvConfig::from_env_with(|k| {
            (k == ENV_OSV_ENDPOINT).then(|| "https://mirror.internal/v1/".to_string())
        })
        .unwrap()
        .unwrap();
        assert_eq!(cfg.base_url, "https://mirror.internal/v1");

        assert_eq!(OsvConfig::from_env_with(|_| None).unwrap(), None);

        let err =
            OsvConfig::from_env_with(|k| (k == ENV_OSV_ENDPOINT).then(|| "ftp://nope".to_string()))
                .unwrap_err();
        assert!(err.contains("http(s)"));
    }

    #[tokio::test]
    async fn test_unreachable_endpoint_is_a_loud_error() {
        // Port 1 on localhost: nothing listens; connection must fail fast.
        let client = OsvClient::new(OsvConfig {
            base_url: "http://127.0.0.1:1/v1".to_string(),
            timeout: Duration::from_millis(500),
        })
        .unwrap();

        let result = client
            .query_packages(&[(
                "crates.io".to_string(),
                "x".to_string(),
                "1.0.0".to_string(),
            )])
            .await;

        assert!(result.is_err(), "silent empty results would hide outages");
    }
}
