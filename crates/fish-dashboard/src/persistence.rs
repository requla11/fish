//! JSONL-backed persistence for dashboard build metrics.
//!
//! Metrics are appended to a `.jsonl` file after every build and loaded on
//! startup, so the dashboard survives process restarts without a database.

use std::io::Write;
use std::path::PathBuf;

use crate::metrics::BuildMetrics;

/// File-backed store that persists [`BuildMetrics`] as JSON Lines.
pub struct PersistentMetricsStore {
    path: PathBuf,
}

impl PersistentMetricsStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Append one build record; creates parent directories on first write.
    pub fn append(&self, metrics: &BuildMetrics) -> std::io::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let line = serde_json::to_string(metrics)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{line}")
    }

    /// Load all recorded builds, oldest first.
    ///
    /// Corrupted lines are skipped rather than aborting the load.
    pub fn load_all(&self) -> Vec<BuildMetrics> {
        let content = match std::fs::read_to_string(&self.path) {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };
        content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }

    /// Load at most `limit` most recent builds.
    pub fn load_recent(&self, limit: usize) -> Vec<BuildMetrics> {
        let all = self.load_all();
        let start = all.len().saturating_sub(limit);
        all[start..].to_vec()
    }
}

/// Team-level aggregated statistics from persisted build history.
#[derive(Debug, Clone, serde::Serialize)]
pub struct TeamStats {
    pub total_builds: usize,
    pub avg_duration_ms: f64,
    pub median_duration_ms: f64,
    pub overall_cache_hit_rate: f64,
    pub total_cache_hits: u64,
    pub total_cache_misses: u64,
    pub successful_builds: usize,
    pub failed_builds: usize,
}

pub fn compute_team_stats(builds: &[BuildMetrics]) -> Option<TeamStats> {
    if builds.is_empty() {
        return None;
    }

    let mut durations: Vec<f64> = builds
        .iter()
        .map(|b| b.duration_ms.unwrap_or(0) as f64)
        .collect();
    durations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let avg_duration_ms = durations.iter().sum::<f64>() / durations.len() as f64;
    let mid = durations.len() / 2;
    let median_duration_ms = if durations.len() % 2 == 1 {
        durations[mid]
    } else {
        (durations[mid - 1] + durations[mid]) / 2.0
    };

    let total_cache_hits: u64 = builds.iter().map(|b| b.cache_stats.hits).sum();
    let total_cache_misses: u64 = builds.iter().map(|b| b.cache_stats.misses).sum();
    let overall_cache_hit_rate = if total_cache_hits + total_cache_misses > 0 {
        total_cache_hits as f64 / (total_cache_hits + total_cache_misses) as f64
    } else {
        0.0
    };

    use crate::metrics::BuildStatus;
    let successful_builds = builds
        .iter()
        .filter(|b| b.status == BuildStatus::Success)
        .count();
    let failed_builds = builds
        .iter()
        .filter(|b| b.status == BuildStatus::Failed)
        .count();

    Some(TeamStats {
        total_builds: builds.len(),
        avg_duration_ms,
        median_duration_ms,
        overall_cache_hit_rate,
        total_cache_hits,
        total_cache_misses,
        successful_builds,
        failed_builds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{BuildStatus, CacheStats};
    use tempfile::tempdir;

    fn sample_build(duration_ms: u64, hits: u64, misses: u64) -> BuildMetrics {
        BuildMetrics {
            build_id: format!("test_{duration_ms}"),
            project_name: "demo".to_string(),
            backend: "rust".to_string(),
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
            duration_ms: Some(duration_ms),
            status: BuildStatus::Success,
            tasks: Vec::new(),
            cache_stats: CacheStats {
                hits,
                misses,
                hit_rate: 0.0,
                bytes_saved: 0,
            },
        }
    }

    #[test]
    fn test_persistence_roundtrip() {
        let dir = tempdir().unwrap();
        let store = PersistentMetricsStore::new(dir.path().join("builds.jsonl"));
        assert!(store.load_all().is_empty());
        store.append(&sample_build(10000, 5, 3)).unwrap();
        store.append(&sample_build(20000, 8, 6)).unwrap();
        let loaded = store.load_all();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].duration_ms, Some(10000));
    }

    #[test]
    fn test_corrupted_lines_skipped() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("builds.jsonl");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let g1 = serde_json::to_string(&sample_build(5000, 1, 0)).unwrap();
        let g2 = serde_json::to_string(&sample_build(15000, 2, 1)).unwrap();
        std::fs::write(&path, format!("{g1}\nNOT_JSON\n{g2}\n")).unwrap();
        let store = PersistentMetricsStore::new(&path);
        assert_eq!(store.load_all().len(), 2);
    }

    #[test]
    fn test_load_recent_limits() {
        let dir = tempdir().unwrap();
        let store = PersistentMetricsStore::new(dir.path().join("b.jsonl"));
        for i in 0..10u64 {
            store.append(&sample_build(i * 100, i, i)).unwrap();
        }
        assert_eq!(store.load_recent(3).len(), 3);
    }

    #[test]
    fn test_team_stats_aggregation() {
        let builds = vec![
            sample_build(10000, 10, 2),
            sample_build(20000, 20, 5),
            sample_build(15000, 15, 3),
        ];
        let stats = compute_team_stats(&builds).unwrap();
        assert_eq!(stats.total_builds, 3);
        assert!((stats.median_duration_ms - 15000.0).abs() < 1e-9);
        assert_eq!(stats.total_cache_hits, 45);
        assert_eq!(stats.total_cache_misses, 10);
    }

    #[test]
    fn test_team_stats_empty_is_none() {
        assert!(compute_team_stats(&[]).is_none());
    }
}
