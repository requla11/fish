#![forbid(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use parking_lot::RwLock;

/// Cache access pattern for ML-based prediction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessPattern {
    /// Sequential access (A, B, C, D...)
    Sequential,
    /// Random access
    Random,
    /// LRU pattern (recently accessed items likely to be accessed again)
    Lru,
    /// LFU pattern (frequently accessed items likely to be accessed again)
    Lfu,
    /// Unknown pattern
    Unknown,
}

/// Cache hit prediction based on access patterns
#[derive(Debug, Clone)]
pub struct CachePrediction {
    pub key: String,
    pub hit_probability: f64,
    pub predicted_next_access: Option<Instant>,
    pub access_pattern: AccessPattern,
    pub access_count: u64,
    pub last_access: Instant,
}

/// ML-based cache predictor
pub struct CachePredictor {
    pub access_history: Arc<RwLock<HashMap<String, VecDeque<Instant>>>>,
    pub access_patterns: Arc<RwLock<HashMap<String, AccessPattern>>>,
    pub hit_counts: Arc<RwLock<HashMap<String, u64>>>,
    pub miss_counts: Arc<RwLock<HashMap<String, u64>>>,
    pub max_history_size: usize,
    pub pattern_detection_threshold: usize,
}

impl CachePredictor {
    pub fn new(max_history_size: usize, pattern_detection_threshold: usize) -> Self {
        Self {
            access_history: Arc::new(RwLock::new(HashMap::new())),
            access_patterns: Arc::new(RwLock::new(HashMap::new())),
            hit_counts: Arc::new(RwLock::new(HashMap::new())),
            miss_counts: Arc::new(RwLock::new(HashMap::new())),
            max_history_size,
            pattern_detection_threshold,
        }
    }

    pub fn with_default_config() -> Self {
        Self::new(100, 10)
    }

    /// Record a cache access
    pub fn record_access(&self, key: &str, hit: bool) {
        let now = Instant::now();

        let mut history = self.access_history.write();
        let history_entry = history.entry(key.to_string()).or_default();
        history_entry.push_back(now);
        if history_entry.len() > self.max_history_size {
            history_entry.pop_front();
        }

        if hit {
            let mut hit_counts = self.hit_counts.write();
            *hit_counts.entry(key.to_string()).or_insert(0) += 1;
        } else {
            let mut miss_counts = self.miss_counts.write();
            *miss_counts.entry(key.to_string()).or_insert(0) += 1;
        }

        if history_entry.len() >= self.pattern_detection_threshold {
            self.detect_pattern(key, history_entry);
        }
    }

    /// Detect access pattern for a key
    #[allow(clippy::manual_checked_ops)]
    fn detect_pattern(&self, key: &str, history: &VecDeque<Instant>) {
        if history.len() < 3 {
            return;
        }

        let intervals: Vec<Duration> = history
            .iter()
            .zip(history.iter().skip(1))
            .map(|(a, b)| b.saturating_duration_since(*a))
            .collect();

        let avg_interval = if intervals.is_empty() {
            Duration::ZERO
        } else {
            let total: Duration = intervals.iter().sum();
            total / intervals.len() as u32
        };

        let variance = if intervals.is_empty() {
            Duration::ZERO
        } else {
            let variance_sum: u64 = intervals
                .iter()
                .map(|i| {
                    let diff = i.as_millis() as i64 - avg_interval.as_millis() as i64;
                    (diff * diff) as u64
                })
                .sum();
            Duration::from_millis(variance_sum / intervals.len() as u64)
        };

        let pattern = if variance < avg_interval / 10 {
            AccessPattern::Sequential
        } else if variance > avg_interval * 10 {
            AccessPattern::Random
        } else {
            let hit_counts = self.hit_counts.read();
            let miss_counts = self.miss_counts.read();
            let hits = *hit_counts.get(key).unwrap_or(&0);
            let misses = *miss_counts.get(key).unwrap_or(&0);
            let total = hits + misses;

            if total > 0 {
                let hit_rate = hits as f64 / total as f64;
                if hit_rate > 0.7 {
                    AccessPattern::Lru
                } else if hit_rate > 0.5 {
                    AccessPattern::Lfu
                } else {
                    AccessPattern::Unknown
                }
            } else {
                AccessPattern::Unknown
            }
        };

        let mut patterns = self.access_patterns.write();
        patterns.insert(key.to_string(), pattern);
    }

    /// Predict cache hit probability and next access time
    #[allow(clippy::manual_checked_ops)]
    pub fn predict(&self, key: &str) -> Option<CachePrediction> {
        let history = self.access_history.read();
        let history_entry = history.get(key)?;

        if history_entry.is_empty() {
            return None;
        }

        let last_access = *history_entry.back()?;
        let access_count = history_entry.len() as u64;

        let hit_counts = self.hit_counts.read();
        let miss_counts = self.miss_counts.read();
        let hits = *hit_counts.get(key).unwrap_or(&0);
        let misses = *miss_counts.get(key).unwrap_or(&0);
        let total = hits + misses;

        let hit_probability = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.5
        };

        let patterns = self.access_patterns.read();
        let access_pattern = *patterns.get(key).unwrap_or(&AccessPattern::Unknown);

        let predicted_next_access = if history_entry.len() >= 2 {
            let intervals: Vec<Duration> = history_entry
                .iter()
                .zip(history_entry.iter().skip(1))
                .map(|(a, b)| b.saturating_duration_since(*a))
                .collect();

            if !intervals.is_empty() {
                let avg_interval: Duration =
                    intervals.iter().sum::<Duration>() / intervals.len() as u32;
                Some(last_access + avg_interval)
            } else {
                None
            }
        } else {
            None
        };

        Some(CachePrediction {
            key: key.to_string(),
            hit_probability,
            predicted_next_access,
            access_pattern,
            access_count,
            last_access,
        })
    }

    /// Get keys that are likely to be accessed soon
    #[allow(clippy::manual_checked_ops)]
    pub fn prefetch_candidates(&self, horizon: Duration) -> Vec<String> {
        let mut candidates = Vec::new();
        let now = Instant::now();

        let history = self.access_history.read();
        let patterns = self.access_patterns.read();

        for (key, history_entry) in history.iter() {
            if history_entry.len() < 2 {
                continue;
            }

            let last_access = *history_entry.back().unwrap();
            let intervals: Vec<Duration> = history_entry
                .iter()
                .zip(history_entry.iter().skip(1))
                .map(|(a, b)| b.saturating_duration_since(*a))
                .collect();

            if intervals.is_empty() {
                continue;
            }

            let avg_interval: Duration =
                intervals.iter().sum::<Duration>() / intervals.len() as u32;
            let next_access = last_access + avg_interval;

            if next_access <= now + horizon {
                let _pattern = *patterns.get(key).unwrap_or(&AccessPattern::Unknown);
                let hit_counts = self.hit_counts.read();
                let misses = self.miss_counts.read();
                let hits = *hit_counts.get(key).unwrap_or(&0);
                let miss = *misses.get(key).unwrap_or(&0);
                let total = hits + miss;

                if total > 0 {
                    let hit_rate = hits as f64 / total as f64;
                    if hit_rate > 0.5 {
                        candidates.push(key.clone());
                    }
                }
            }
        }

        candidates.sort_by(|a, b| {
            let pred_a = self.predict(a).map(|p| p.hit_probability).unwrap_or(0.0);
            let pred_b = self.predict(b).map(|p| p.hit_probability).unwrap_or(0.0);
            pred_b
                .partial_cmp(&pred_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates
    }

    /// Get cache statistics
    #[allow(clippy::manual_checked_ops)]
    pub fn stats(&self) -> CachePredictorStats {
        let history = self.access_history.read();
        let hit_counts = self.hit_counts.read();
        let miss_counts = self.miss_counts.read();

        let total_keys = history.len();
        let total_accesses: u64 = history.values().map(|h| h.len() as u64).sum();
        let total_hits: u64 = hit_counts.values().sum();
        let total_misses: u64 = miss_counts.values().sum();

        let hit_rate = if total_hits + total_misses > 0 {
            total_hits as f64 / (total_hits + total_misses) as f64
        } else {
            0.0
        };

        CachePredictorStats {
            total_keys,
            total_accesses,
            total_hits,
            total_misses,
            hit_rate,
        }
    }

    /// Clear all history
    pub fn clear(&self) {
        self.access_history.write().clear();
        self.access_patterns.write().clear();
        self.hit_counts.write().clear();
        self.miss_counts.write().clear();
    }
}

impl Default for CachePredictor {
    fn default() -> Self {
        Self::with_default_config()
    }
}

/// Cache predictor statistics
#[derive(Debug, Clone)]
pub struct CachePredictorStats {
    pub total_keys: usize,
    pub total_accesses: u64,
    pub total_hits: u64,
    pub total_misses: u64,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_predictor_creation() {
        let predictor = CachePredictor::with_default_config();
        assert_eq!(predictor.max_history_size, 100);
        assert_eq!(predictor.pattern_detection_threshold, 10);
    }

    #[test]
    fn test_record_access() {
        let predictor = CachePredictor::with_default_config();
        predictor.record_access("key1", true);
        predictor.record_access("key1", false);

        let history = predictor.access_history.read();
        assert_eq!(history.get("key1").unwrap().len(), 2);

        let hit_counts = predictor.hit_counts.read();
        assert_eq!(*hit_counts.get("key1").unwrap(), 1);

        let miss_counts = predictor.miss_counts.read();
        assert_eq!(*miss_counts.get("key1").unwrap(), 1);
    }

    #[test]
    fn test_predict_basic() {
        let predictor = CachePredictor::with_default_config();
        predictor.record_access("key1", true);
        predictor.record_access("key1", true);
        predictor.record_access("key1", false);

        let prediction = predictor.predict("key1");
        assert!(prediction.is_some());

        let pred = prediction.unwrap();
        assert_eq!(pred.key, "key1");
        assert!(pred.hit_probability > 0.0);
        assert_eq!(pred.access_count, 3);
    }

    #[test]
    fn test_predict_unknown_key() {
        let predictor = CachePredictor::with_default_config();
        let prediction = predictor.predict("unknown");
        assert!(prediction.is_none());
    }

    #[test]
    fn test_prefetch_candidates() {
        let predictor = CachePredictor::new(10, 3);

        for _i in 0..5 {
            predictor.record_access("key1", true);
            std::thread::sleep(Duration::from_millis(10));
        }

        let candidates = predictor.prefetch_candidates(Duration::from_secs(1));
        assert!(!candidates.is_empty());
    }

    #[test]
    fn test_stats() {
        let predictor = CachePredictor::with_default_config();
        predictor.record_access("key1", true);
        predictor.record_access("key1", false);
        predictor.record_access("key2", true);

        let stats = predictor.stats();
        assert_eq!(stats.total_keys, 2);
        assert_eq!(stats.total_accesses, 3);
        assert_eq!(stats.total_hits, 2);
        assert_eq!(stats.total_misses, 1);
        assert!(stats.hit_rate > 0.0);
    }

    #[test]
    fn test_clear() {
        let predictor = CachePredictor::with_default_config();
        predictor.record_access("key1", true);
        predictor.record_access("key2", false);

        predictor.clear();

        let history = predictor.access_history.read();
        assert!(history.is_empty());

        let stats = predictor.stats();
        assert_eq!(stats.total_keys, 0);
    }

    #[test]
    fn test_history_size_limit() {
        let predictor = CachePredictor::new(5, 3);

        for _i in 0..10 {
            predictor.record_access("key1", true);
            std::thread::sleep(Duration::from_millis(1));
        }

        let history = predictor.access_history.read();
        assert!(history.get("key1").unwrap().len() <= 5);
    }
}
