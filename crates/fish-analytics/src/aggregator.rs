use crate::metrics::CacheMetrics;
use std::path::Path;

#[derive(Clone, Default)]
pub struct MetricsAggregator;

impl MetricsAggregator {
    pub fn new() -> Self {
        Self
    }

    pub async fn collect(&self, project_path: &Path) -> Result<CacheMetrics, anyhow::Error> {
        let fish_dir = project_path.join(".fish");
        let cache_dir = fish_dir.join("cache");
        let cas_dir = fish_dir.join("cas");
        let metrics_file = fish_dir.join("metrics").join("summary.json");

        let mut cache_size_bytes = 0u64;
        for dir in &[&cache_dir, &cas_dir] {
            if dir.exists() {
                cache_size_bytes += Self::calculate_dir_size(dir);
            }
        }

        let mut total_hits = 0u64;
        let mut total_misses = 0u64;
        let mut total_requests = 0u64;

        if metrics_file.exists()
            && let Ok(content) = tokio::fs::read_to_string(&metrics_file).await
            && let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&content)
        {
            total_hits = parsed
                .get("total_hits")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            total_misses = parsed
                .get("total_misses")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            total_requests = parsed
                .get("total_requests")
                .and_then(|v| v.as_u64())
                .unwrap_or(total_hits + total_misses);
        }

        let hit_rate = if total_requests > 0 {
            (total_hits as f64) / (total_requests as f64)
        } else {
            0.0
        };

        Ok(CacheMetrics {
            hit_rate,
            total_hits,
            total_misses,
            total_requests,
            cache_size_bytes,
            timestamp: chrono::Utc::now(),
        })
    }

    fn calculate_dir_size(dir: &Path) -> u64 {
        let mut size = 0;
        let mut stack = vec![dir.to_path_buf()];
        while let Some(current) = stack.pop() {
            if let Ok(entries) = std::fs::read_dir(&current) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else if let Ok(meta) = entry.metadata() {
                        size += meta.len();
                    }
                }
            }
        }
        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_collect_real_cache_size_and_metrics() {
        let temp = tempdir().unwrap();
        let fish_dir = temp.path().join(".fish");
        let cache_dir = fish_dir.join("cache");
        let metrics_dir = fish_dir.join("metrics");

        std::fs::create_dir_all(&cache_dir).unwrap();
        std::fs::create_dir_all(&metrics_dir).unwrap();

        std::fs::write(cache_dir.join("artifact.bin"), vec![0u8; 1024]).unwrap();

        let summary = serde_json::json!({
            "total_hits": 80,
            "total_misses": 20,
            "total_requests": 100
        });
        std::fs::write(metrics_dir.join("summary.json"), summary.to_string()).unwrap();

        let aggregator = MetricsAggregator::new();
        let metrics = aggregator.collect(temp.path()).await.unwrap();

        assert_eq!(metrics.total_hits, 80);
        assert_eq!(metrics.total_misses, 20);
        assert_eq!(metrics.total_requests, 100);
        assert!((metrics.hit_rate - 0.8).abs() < f64::EPSILON);
        assert_eq!(metrics.cache_size_bytes, 1024);
    }
}
