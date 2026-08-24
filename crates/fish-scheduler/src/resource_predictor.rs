use std::collections::HashMap;

/// A historical observation of a task's resource footprint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResourceSample {
    pub peak_ram_bytes: u64,
    pub duration_secs: f64,
}

/// Learned per-task resource predictions from history.
///
/// Uses order-statistic percentiles (no ML dependency): the predicted
/// allocation for a task is its P90 historical peak RAM, so 9 of 10 runs
/// fit without OOM-kill churn. Cold tasks fall back to a configurable
/// conservative default.
#[derive(Debug, Default)]
pub struct LearnedResourcePredictor {
    history: HashMap<String, Vec<ResourceSample>>,
    max_samples_per_task: usize,
}

impl LearnedResourcePredictor {
    /// Keep at most `max_samples` most-recent samples per task.
    pub fn new(max_samples: usize) -> Self {
        Self {
            history: HashMap::new(),
            max_samples_per_task: max_samples,
        }
    }

    /// Record a completed task run.
    pub fn observe(&mut self, task_key: &str, sample: ResourceSample) {
        let bucket = self.history.entry(task_key.to_string()).or_default();
        bucket.push(sample);
        if bucket.len() > self.max_samples_per_task {
            // Drop oldest.
            bucket.remove(0);
        }
    }

    /// Predicted memory budget (P90) and duration estimate (median).
    ///
    /// Returns `None` when there is no history for the task.
    pub fn predict(&self, task_key: &str) -> Option<Prediction> {
        let samples = self.history.get(task_key)?;
        if samples.is_empty() {
            return None;
        }
        Some(Prediction {
            p90_peak_ram_bytes: percentile(
                samples.iter().map(|s| s.peak_ram_bytes).collect(),
                0.90,
            ),
            median_duration_secs: percentile_f64(
                samples.iter().map(|s| s.duration_secs).collect(),
                0.50,
            ),
            sample_count: samples.len(),
        })
    }

    /// Whether this task has enough samples to trust predictions.
    pub fn is_warm(&self, task_key: &str, min_samples: usize) -> bool {
        self.history
            .get(task_key)
            .is_some_and(|v| v.len() >= min_samples)
    }

    /// Number of tracked tasks.
    pub fn tracked_tasks(&self) -> usize {
        self.history.len()
    }
}

/// Prediction output for a single task key.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Prediction {
    pub p90_peak_ram_bytes: u64,
    pub median_duration_secs: f64,
    pub sample_count: usize,
}

/// Nearest-rank percentile over a copied list.
fn percentile(mut values: Vec<u64>, q: f64) -> u64 {
    values.sort_unstable();
    nearest_rank(values.as_slice(), q, |v| *v)
}

/// Percentile over durations.
fn percentile_f64(mut values: Vec<f64>, q: f64) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    nearest_rank(values.as_slice(), q, |v| *v)
}

fn nearest_rank<T: Copy>(sorted: &[T], q: f64, get: impl Fn(&T) -> T) -> T {
    debug_assert!(!sorted.is_empty());
    let rank = ((q * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    get(&sorted[rank - 1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cold_task_predicts_none() {
        let predictor = LearnedResourcePredictor::new(50);
        assert!(predictor.predict("never-seen").is_none());
        assert!(!predictor.is_warm("never-seen", 3));
    }

    #[test]
    fn p90_uses_nearest_rank() {
        let mut p = LearnedResourcePredictor::new(100);
        // 10 samples: 10..100 MB; P90 rank = ceil(0.9*10)=9 → 90 MB
        for i in 1..=10u64 {
            p.observe(
                "build",
                ResourceSample {
                    peak_ram_bytes: i * 10 * 1024 * 1024,
                    duration_secs: i as f64,
                },
            );
        }
        let pred = p.predict("build").unwrap();
        assert_eq!(pred.p90_peak_ram_bytes, 90 * 1024 * 1024);
        // median rank = ceil(0.5*10)=5 → 5.0s
        assert!((pred.median_duration_secs - 5.0).abs() < 1e-9);
        assert!(p.is_warm("build", 3));
    }

    #[test]
    fn ring_buffer_drops_oldest() {
        let mut p = LearnedResourcePredictor::new(3);
        for i in 0..5u64 {
            p.observe(
                "t",
                ResourceSample {
                    peak_ram_bytes: i,
                    duration_secs: 1.0,
                },
            );
        }
        let pred = p.predict("t").unwrap();
        assert_eq!(pred.sample_count, 3);
        // Oldest two dropped → remaining {2,3,4}; P90 = ceil(2.7)=3rd = 4
        assert_eq!(pred.p90_peak_ram_bytes, 4);
    }

    #[test]
    fn multiple_tasks_tracked_independently() {
        let mut p = LearnedResourcePredictor::new(10);
        p.observe(
            "a",
            ResourceSample {
                peak_ram_bytes: 1,
                duration_secs: 1.0,
            },
        );
        p.observe(
            "b",
            ResourceSample {
                peak_ram_bytes: 999,
                duration_secs: 2.0,
            },
        );
        assert_eq!(p.tracked_tasks(), 2);
        assert_eq!(p.predict("a").unwrap().p90_peak_ram_bytes, 1);
        assert_eq!(p.predict("b").unwrap().p90_peak_ram_bytes, 999);
    }
}
