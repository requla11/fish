use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildRunRecord {
    pub timestamp_unix_secs: u64,
    pub duration_secs: f64,
    pub tasks_total: usize,
    pub tasks_failed: usize,
}

/// Thresholds governing when a build counts as a regression.
///
/// A regression requires BOTH conditions: the run is slower than the
/// baseline by at least [`Self::relative_threshold_pct`] percent AND the
/// overshoot exceeds [`Self::absolute_floor_secs`] seconds, so sub-second
/// noise on tiny builds never triggers alerts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionConfig {
    pub relative_threshold_pct: f64,
    pub absolute_floor_secs: f64,
    /// How many past runs form the rolling history.
    pub history_limit: usize,
}

impl Default for RegressionConfig {
    fn default() -> Self {
        Self {
            relative_threshold_pct: 20.0,
            absolute_floor_secs: 5.0,
            history_limit: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegressionHistory {
    pub runs: Vec<BuildRunRecord>,
}

impl RegressionHistory {
    pub fn load(path: &Path) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, content)
    }

    /// Append a run, trimming history to the configured limit. Returns the
    /// number of dropped records.
    pub fn record(&mut self, run: BuildRunRecord, limit: usize) -> usize {
        self.runs.push(run);
        let overflow = self.runs.len().saturating_sub(limit);
        if overflow > 0 {
            self.runs.drain(..overflow);
        }
        overflow
    }

    /// Baseline is the median duration of prior runs — robust against one
    /// pathological outlier dominating the comparison.
    pub fn baseline_median_secs(&self) -> Option<f64> {
        let mut durations: Vec<f64> = self.runs.iter().map(|r| r.duration_secs).collect();
        if durations.is_empty() {
            return None;
        }
        durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = durations.len() / 2;
        Some(if durations.len() % 2 == 1 {
            durations[mid]
        } else {
            (durations[mid - 1] + durations[mid]) / 2.0
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RegressionVerdict {
    /// Fewer than three recorded runs; no statistically meaningful baseline.
    InsufficientHistory,
    Healthy {
        baseline_secs: f64,
    },
    Regressed {
        baseline_secs: f64,
        overshoot_pct: f64,
    },
    Improved {
        baseline_secs: f64,
        improvement_pct: f64,
    },
}

pub fn evaluate(
    current_duration_secs: f64,
    history: &RegressionHistory,
    config: &RegressionConfig,
) -> RegressionVerdict {
    if history.runs.len() < 3 {
        return RegressionVerdict::InsufficientHistory;
    }
    let baseline = history
        .baseline_median_secs()
        .expect("history checked non-empty");
    if baseline <= 0.0 {
        return RegressionVerdict::InsufficientHistory;
    }

    let delta_pct = (current_duration_secs - baseline) / baseline * 100.0;
    if delta_pct > config.relative_threshold_pct
        && current_duration_secs - baseline > config.absolute_floor_secs
    {
        RegressionVerdict::Regressed {
            baseline_secs: baseline,
            overshoot_pct: delta_pct,
        }
    } else if delta_pct < -(config.relative_threshold_pct) {
        RegressionVerdict::Improved {
            baseline_secs: baseline,
            improvement_pct: -delta_pct,
        }
    } else {
        RegressionVerdict::Healthy {
            baseline_secs: baseline,
        }
    }
}

/// Standard location of the per-project history file.
pub fn default_history_path(project_root: &Path) -> PathBuf {
    project_root
        .join(".fish")
        .join("metrics")
        .join("builds.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(secs: f64) -> BuildRunRecord {
        BuildRunRecord {
            timestamp_unix_secs: 0,
            duration_secs: secs,
            tasks_total: 10,
            tasks_failed: 0,
        }
    }

    #[test]
    fn test_insufficient_history_below_three_runs() {
        let cfg = RegressionConfig::default();
        let mut h = RegressionHistory::default();
        assert_eq!(
            evaluate(100.0, &h, &cfg),
            RegressionVerdict::InsufficientHistory
        );
        h.runs.push(run(90.0));
        h.runs.push(run(95.0));
        assert_eq!(
            evaluate(100.0, &h, &cfg),
            RegressionVerdict::InsufficientHistory
        );
    }

    #[test]
    fn test_median_baseline_ignores_outlier() {
        let h = RegressionHistory {
            runs: vec![run(50.0), run(52.0), run(48.0), run(500.0)],
        };
        // median of {48,50,52,500} = (50+52)/2 = 51
        assert!((h.baseline_median_secs().unwrap() - 51.0).abs() < 1e-9);
    }

    #[test]
    fn test_regression_requires_both_thresholds() {
        let cfg = RegressionConfig {
            relative_threshold_pct: 20.0,
            absolute_floor_secs: 5.0,
            history_limit: 10,
        };
        let h = RegressionHistory {
            runs: vec![run(100.0), run(100.0), run(100.0)],
        };

        // +30% but only 3s absolute: below floor, healthy
        assert!(matches!(
            evaluate(103.0, &h, &cfg),
            RegressionVerdict::Healthy { .. }
        ));

        // +2% = 2s on a 100s baseline: above floor? no -> healthy
        assert!(matches!(
            evaluate(102.0, &h, &cfg),
            RegressionVerdict::Healthy { .. }
        ));

        // +25% and +25s: both conditions met -> regressed
        match evaluate(125.0, &h, &cfg) {
            RegressionVerdict::Regressed { overshoot_pct, .. } => {
                assert!((overshoot_pct - 25.0).abs() < 1e-9);
            }
            other => panic!("expected regression, got {other:?}"),
        }
    }

    #[test]
    fn test_improvement_is_reported() {
        let cfg = RegressionConfig::default();
        let h = RegressionHistory {
            runs: vec![run(200.0), run(200.0), run(200.0)],
        };

        match evaluate(150.0, &h, &cfg) {
            RegressionVerdict::Improved {
                improvement_pct, ..
            } => {
                assert!((improvement_pct - 25.0).abs() < 1e-9);
            }
            other => panic!("expected improvement, got {other:?}"),
        }
    }

    #[test]
    fn test_record_trims_history_to_limit() {
        let mut h = RegressionHistory::default();
        for i in 0..12 {
            h.record(run(i as f64), 10);
        }
        assert_eq!(h.runs.len(), 10);
        assert!((h.runs[0].duration_secs - 2.0).abs() < 1e-9);
    }

    #[test]
    fn test_history_roundtrips_through_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("metrics").join("builds.json");
        assert!(!path.exists());

        let loaded = RegressionHistory::load(&path).unwrap();
        assert!(loaded.runs.is_empty());

        let mut h = RegressionHistory::default();
        h.record(run(42.0), 5);
        h.save(&path).unwrap();

        let reloaded = RegressionHistory::load(&path).unwrap();
        assert_eq!(reloaded.runs.len(), 1);
        assert!((reloaded.runs[0].duration_secs - 42.0).abs() < 1e-9);
    }
}
