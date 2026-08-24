use anyhow::{Context, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

/// A single build run stored in the time-series database.
#[derive(Debug, Clone, PartialEq)]
pub struct BuildTimeSeriesRow {
    pub timestamp_unix_secs: u64,
    pub duration_secs: f64,
    pub tasks_total: usize,
    pub tasks_failed: usize,
    pub project: String,
    pub branch: String,
}

/// SQLite-backed time-series store for build metrics.
///
/// Replaces the append-only JSONL history with a real queryable
/// database supporting per-project/branch/time-window aggregation.
pub struct TimeSeriesStore {
    conn: Connection,
}

impl TimeSeriesStore {
    /// Open (or create) the store at `path`.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let conn = Connection::open(path)
            .with_context(|| format!("failed to open sqlite at {}", path.display()))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             CREATE TABLE IF NOT EXISTS build_runs (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 ts_unix_secs INTEGER NOT NULL,
                 duration_secs REAL NOT NULL,
                 tasks_total INTEGER NOT NULL,
                 tasks_failed INTEGER NOT NULL,
                 project TEXT NOT NULL DEFAULT '',
                 branch TEXT NOT NULL DEFAULT ''
             );
             CREATE INDEX IF NOT EXISTS idx_build_runs_ts ON build_runs(ts_unix_secs);
             CREATE INDEX IF NOT EXISTS idx_build_runs_proj ON build_runs(project, branch);",
        )?;
        Ok(Self { conn })
    }

    /// Default location inside the project's fish cache directory.
    pub fn default_path(fish_dir: &Path) -> PathBuf {
        fish_dir.join("metrics").join("builds.sqlite")
    }

    /// Insert one build record.
    pub fn insert(&self, row: &BuildTimeSeriesRow) -> Result<()> {
        self.conn
            .execute(
                "INSERT INTO build_runs (ts_unix_secs, duration_secs, tasks_total, tasks_failed, project, branch)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                rusqlite::params![
                    row.timestamp_unix_secs as i64,
                    row.duration_secs,
                    row.tasks_total as i64,
                    row.tasks_failed as i64,
                    row.project,
                    row.branch,
                ],
            )
            .context("insert build run")?;
        Ok(())
    }

    /// Duration statistics over an optional filter window.
    ///
    /// An empty project string matches all projects; `since_secs == 0`
    /// disables the time filter.
    pub fn stats(&self, project: Option<&str>, since_secs: Option<u64>) -> Result<DurationStats> {
        let sql = "SELECT COUNT(*), AVG(duration_secs), MIN(duration_secs), MAX(duration_secs),
                          SUM(tasks_total), SUM(tasks_failed)
                   FROM build_runs
                   WHERE (?1 = '' OR project = ?1)
                     AND (?2 = 0 OR ts_unix_secs >= ?2)";
        let mut stmt = self.conn.prepare(sql)?;
        let proj = project.unwrap_or("");
        let since = since_secs.unwrap_or(0) as i64;
        let row = stmt.query_row(rusqlite::params![proj, since], |r| {
            Ok(DurationStats {
                runs: r.get::<_, i64>(0)?.max(0) as usize,
                mean_secs: r.get(1)?,
                min_secs: r.get(2)?,
                max_secs: r.get(3)?,
                total_tasks: r.get::<_, Option<i64>>(4)?.unwrap_or(0).max(0) as usize,
                total_failed: r.get::<_, Option<i64>>(5)?.unwrap_or(0).max(0) as usize,
            })
        })?;
        Ok(row)
    }

    /// Per-day aggregated durations, newest first.
    pub fn daily_rollup(&self, days: u32) -> Result<Vec<DailyRollup>> {
        let cutoff = now_secs().saturating_sub(u64::from(days) * 86_400);
        let mut stmt = self.conn.prepare(
            "SELECT date(ts_unix_secs, 'unixepoch') AS day,
                    COUNT(*), AVG(duration_secs), MAX(duration_secs)
             FROM build_runs
             WHERE ts_unix_secs >= ?1
             GROUP BY day ORDER BY day DESC",
        )?;
        let rows = stmt
            .query_map([cutoff as i64], |r| {
                Ok(DailyRollup {
                    day: r.get(0)?,
                    runs: r.get::<_, i64>(1)?.max(0) as usize,
                    mean_secs: r.get(2)?,
                    peak_secs: r.get(3)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Slowest builds in the window — useful for spotting outliers.
    pub fn slowest(&self, project: Option<&str>, limit: usize) -> Result<Vec<SlowRun>> {
        let sql = "SELECT ts_unix_secs, duration_secs, branch
                   FROM build_runs
                   WHERE (?1 = '' OR project = ?1)
                   ORDER BY duration_secs DESC LIMIT ?2";
        let mut stmt = self.conn.prepare(sql)?;
        let proj = project.unwrap_or("");
        let rows = stmt
            .query_map(rusqlite::params![proj, limit as i64], |r| {
                Ok(SlowRun {
                    timestamp_unix_secs: r.get::<_, i64>(0)?.max(0) as u64,
                    duration_secs: r.get(1)?,
                    branch: r.get(2)?,
                })
            })?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

/// Aggregate duration statistics.
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct DurationStats {
    pub runs: usize,
    pub mean_secs: Option<f64>,
    pub min_secs: Option<f64>,
    pub max_secs: Option<f64>,
    pub total_tasks: usize,
    pub total_failed: usize,
}

/// One day of rolled-up build activity.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DailyRollup {
    pub day: String,
    pub runs: usize,
    pub mean_secs: Option<f64>,
    pub peak_secs: Option<f64>,
}

/// A single outlier build run.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SlowRun {
    pub timestamp_unix_secs: u64,
    pub duration_secs: f64,
    pub branch: String,
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("ts").join("builds.sqlite");
        (dir, path)
    }

    #[test]
    fn insert_and_stats_roundtrip() {
        let (_guard, path) = temp_db();
        let store = TimeSeriesStore::open(&path).unwrap();

        for i in 0..5u64 {
            store
                .insert(&BuildTimeSeriesRow {
                    timestamp_unix_secs: 1_700_000_000 + i,
                    duration_secs: 10.0 + i as f64,
                    tasks_total: 20,
                    tasks_failed: 0,
                    project: "core".into(),
                    branch: "dev".into(),
                })
                .unwrap();
        }
        let stats = store.stats(Some("core"), None).unwrap();
        assert_eq!(stats.runs, 5);
        assert!((stats.mean_secs.unwrap() - 12.0).abs() < 0.01);
        assert_eq!(stats.min_secs, Some(10.0));
        assert_eq!(stats.max_secs, Some(14.0));
    }

    #[test]
    fn empty_store_returns_zeroes() {
        let (_guard, path) = temp_db();
        let store = TimeSeriesStore::open(&path).unwrap();
        let stats = store.stats(None, None).unwrap();
        assert_eq!(stats.runs, 0);
        assert!(stats.mean_secs.is_none());
    }

    #[test]
    fn daily_rollup_groups_by_day() {
        let (_guard, path) = temp_db();
        let store = TimeSeriesStore::open(&path).unwrap();
        store
            .insert(&BuildTimeSeriesRow {
                timestamp_unix_secs: now_secs(),
                duration_secs: 5.0,
                tasks_total: 4,
                tasks_failed: 0,
                project: String::new(),
                branch: "main".into(),
            })
            .unwrap();
        let rollups = store.daily_rollup(30).unwrap();
        assert_eq!(rollups.len(), 1);
        assert_eq!(rollups[0].runs, 1);
        assert_eq!(rollups[0].peak_secs, Some(5.0));
    }
}
