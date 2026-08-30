use serde::Deserialize;
use std::time::Duration;

/// Carbon intensity of the local grid, in grams CO2eq per kWh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarbonIntensity {
    pub grams_per_kwh: f64,
}

impl CarbonIntensity {
    /// Qualitative band used for scheduling decisions.
    pub fn band(&self) -> CarbonBand {
        match self.grams_per_kwh {
            g if g < 200.0 => CarbonBand::Green,
            g if g < 400.0 => CarbonBand::Moderate,
            _ => CarbonBand::High,
        }
    }
}

/// Qualitative grid cleanliness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarbonBand {
    Green,
    Moderate,
    High,
}

/// Minimal ElectricityMaps-compatible API client.
///
/// Endpoint shape: `GET {base}/carbon-intensity/latest?zone={zone}`
/// with `auth-token` header when a token is configured.
pub struct CarbonClient {
    base_url: String,
    token: Option<String>,
    offline: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LatestResponse {
    carbon_intensity: f64,
}

impl CarbonClient {
    pub fn new(base_url: impl Into<String>, token: Option<String>) -> Self {
        let offline = std::env::var("FISH_OFFLINE")
            .map(|v| {
                v == "1"
                    || v.eq_ignore_ascii_case("true")
                    || v.eq_ignore_ascii_case("yes")
                    || v.eq_ignore_ascii_case("on")
            })
            .unwrap_or(false);
        Self {
            base_url: base_url.into(),
            token,
            offline,
        }
    }

    pub fn with_offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    /// Client honoring `FISH_CARBON_ENDPOINT` (and optional
    /// `FISH_CARBON_TOKEN`). Returns `None` when unset — callers treat
    /// that as "carbon awareness disabled".
    pub fn from_env() -> Option<Self> {
        Self::from_parts(
            std::env::var("FISH_CARBON_ENDPOINT").ok(),
            std::env::var("FISH_CARBON_TOKEN").ok(),
        )
    }

    /// Pure constructor from optional parts — testable without env access.
    pub fn from_parts(base: Option<String>, token: Option<String>) -> Option<Self> {
        base.map(|base| Self::new(base, token))
    }

    /// Fetch current grid intensity for `zone` (e.g. "DE", "US-CAL-CISO").
    pub fn latest_intensity(&self, zone: &str) -> Result<CarbonIntensity, String> {
        let env_offline = std::env::var("FISH_OFFLINE")
            .map(|v| {
                v == "1"
                    || v.eq_ignore_ascii_case("true")
                    || v.eq_ignore_ascii_case("yes")
                    || v.eq_ignore_ascii_case("on")
            })
            .unwrap_or(false);
        if self.offline || env_offline {
            return Err(
                "offline mode enabled (FISH_OFFLINE); grid carbon lookup rejected".to_string(),
            );
        }

        let url = format!(
            "{}/carbon-intensity/latest?zone={}",
            self.base_url.trim_end_matches('/'),
            zone
        );
        let mut req = ureq::get(&url).timeout(Duration::from_secs(5));
        if let Some(token) = &self.token {
            req = req.set("Authorization", &format!("Bearer {token}"));
        }
        let resp = req.call().map_err(|e| format!("request failed: {e}"))?;
        let body: LatestResponse = resp
            .into_json()
            .map_err(|e| format!("bad response body: {e}"))?;
        Ok(CarbonIntensity {
            grams_per_kwh: body.carbon_intensity,
        })
    }
}

/// Scheduling decision derived from grid state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarbonDecision {
    /// Grid is clean — run everything including deferrable work.
    RunAll,
    /// Moderate — run critical path, defer non-critical batch jobs.
    DeferNonCritical,
    /// Dirty grid — defer everything not on the critical path.
    DeferAllOptional,
}

/// Map an intensity to a policy. Pure function for testability.
pub fn decide(intensity: &CarbonIntensity) -> CarbonDecision {
    match intensity.band() {
        CarbonBand::Green => CarbonDecision::RunAll,
        CarbonBand::Moderate => CarbonDecision::DeferNonCritical,
        CarbonBand::High => CarbonDecision::DeferAllOptional,
    }
}

/// Whether a task with the given priority may start under this decision.
pub fn may_run(decision: &CarbonDecision, task_priority: u8) -> bool {
    // Higher priority number == more critical.
    match decision {
        CarbonDecision::RunAll => true,
        CarbonDecision::DeferNonCritical => task_priority >= 5,
        CarbonDecision::DeferAllOptional => task_priority >= 8,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_map_to_decisions() {
        assert_eq!(
            decide(&CarbonIntensity {
                grams_per_kwh: 50.0
            }),
            CarbonDecision::RunAll
        );
        assert_eq!(
            decide(&CarbonIntensity {
                grams_per_kwh: 300.0
            }),
            CarbonDecision::DeferNonCritical
        );
        assert_eq!(
            decide(&CarbonIntensity {
                grams_per_kwh: 600.0
            }),
            CarbonDecision::DeferAllOptional
        );
    }

    #[test]
    fn priority_gates() {
        assert!(may_run(&CarbonDecision::RunAll, 0));
        assert!(!may_run(&CarbonDecision::DeferNonCritical, 3));
        assert!(may_run(&CarbonDecision::DeferNonCritical, 7));
        assert!(!may_run(&CarbonDecision::DeferAllOptional, 7));
        assert!(may_run(&CarbonDecision::DeferAllOptional, 9));
    }

    #[test]
    fn client_requires_env() {
        assert!(CarbonClient::from_parts(None, None).is_none());
        let client = CarbonClient::from_parts(Some("http://x".into()), Some("t".into()));
        assert!(client.is_some());
    }

    #[test]
    fn bad_endpoint_errors_cleanly() {
        let client = CarbonClient::new("http://127.0.0.1:1", None);
        let err = client.latest_intensity("DE").unwrap_err();
        assert!(err.contains("request failed"));
    }

    #[test]
    fn band_boundaries() {
        assert_eq!(
            CarbonIntensity {
                grams_per_kwh: 199.9
            }
            .band(),
            CarbonBand::Green
        );
        assert_eq!(
            CarbonIntensity {
                grams_per_kwh: 200.0
            }
            .band(),
            CarbonBand::Moderate
        );
        assert_eq!(
            CarbonIntensity {
                grams_per_kwh: 400.0
            }
            .band(),
            CarbonBand::High
        );
    }

    #[test]
    fn test_offline_carbon_lookup_fail_fast() {
        let client = CarbonClient::new("http://example.com", None).with_offline(true);
        let err = client.latest_intensity("DE").unwrap_err();
        assert!(err.contains("offline mode"));
    }
}
