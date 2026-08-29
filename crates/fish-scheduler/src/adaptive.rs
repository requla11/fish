#![forbid(unsafe_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::resource_governor::{KernelResourceGovernor, MemoryPressureLevel};
use super::resource_predictor::{LearnedResourcePredictor, Prediction, ResourceSample};

/// Workload type classification for adaptive scheduling
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadType {
    /// CPU-intensive tasks (compilation, optimization)
    CpuBound,
    /// I/O-intensive tasks (file operations, network)
    IoBound,
    /// Mixed workload
    Mixed,
    /// Unknown (not enough data)
    Unknown,
}

/// Adaptive parallelism configuration
#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    /// Base number of workers (default: CPU count)
    pub base_workers: usize,
    /// Maximum workers (default: 2x CPU count)
    pub max_workers: usize,
    /// Minimum workers (default: 1)
    pub min_workers: usize,
    /// How often to re-evaluate parallelism (default: 1s)
    pub reevaluation_interval: Duration,
    /// CPU utilization target (0.0-1.0, default: 0.85)
    pub cpu_target: f64,
    /// Memory pressure threshold for throttling (default: 85%)
    pub memory_threshold: u8,
    /// Minimum samples before considering workload type (default: 5)
    pub min_samples_for_classification: usize,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        let cpu_count = num_cpus::get();
        Self {
            base_workers: cpu_count,
            max_workers: (cpu_count * 2).max(4),
            min_workers: 1,
            reevaluation_interval: Duration::from_secs(1),
            cpu_target: 0.85,
            memory_threshold: 85,
            min_samples_for_classification: 5,
        }
    }
}

/// Performance metrics for adaptive scheduling
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Average task duration
    pub avg_task_duration: Duration,
    /// Task throughput (tasks/second)
    pub throughput: f64,
    /// CPU utilization estimate (0.0-1.0)
    pub cpu_utilization: f64,
    /// Memory pressure level
    pub memory_pressure: MemoryPressureLevel,
    /// Current worker count
    pub current_workers: usize,
    /// Time since last adjustment
    pub time_since_adjustment: Duration,
}

/// Adaptive parallelism scheduler
pub struct AdaptiveParallelismScheduler {
    config: AdaptiveConfig,
    resource_governor: KernelResourceGovernor,
    resource_predictor: Arc<parking_lot::Mutex<LearnedResourcePredictor>>,
    current_workers: Arc<AtomicUsize>,
    last_adjustment: Arc<parking_lot::Mutex<Instant>>,
    completed_tasks: Arc<AtomicU64>,
    total_task_time: Arc<AtomicU64>,
    start_time: Instant,
    workload_type: Arc<parking_lot::RwLock<WorkloadType>>,
}

impl AdaptiveParallelismScheduler {
    pub fn new(config: AdaptiveConfig) -> Self {
        let base_workers = config
            .base_workers
            .max(config.min_workers)
            .min(config.max_workers);
        let memory_threshold = config.memory_threshold;
        let reevaluation_interval = config.reevaluation_interval;

        Self {
            config,
            resource_governor: KernelResourceGovernor::new(None, Some(memory_threshold)),
            resource_predictor: Arc::new(parking_lot::Mutex::new(LearnedResourcePredictor::new(
                100,
            ))),
            current_workers: Arc::new(AtomicUsize::new(base_workers)),
            last_adjustment: Arc::new(parking_lot::Mutex::new(
                Instant::now() - reevaluation_interval - Duration::from_secs(1),
            )),
            completed_tasks: Arc::new(AtomicU64::new(0)),
            total_task_time: Arc::new(AtomicU64::new(0)),
            start_time: Instant::now(),
            workload_type: Arc::new(parking_lot::RwLock::new(WorkloadType::Unknown)),
        }
    }

    pub fn with_resource_predictor(
        mut self,
        predictor: Arc<parking_lot::Mutex<LearnedResourcePredictor>>,
    ) -> Self {
        self.resource_predictor = predictor;
        self
    }

    /// Get current optimal worker count
    pub fn optimal_workers(&self) -> usize {
        self.current_workers.load(Ordering::Relaxed)
    }

    /// Record a completed task for adaptive learning
    pub fn record_task(&self, task_key: &str, duration: Duration, peak_ram_bytes: u64) {
        let sample = ResourceSample {
            peak_ram_bytes,
            duration_secs: duration.as_secs_f64(),
        };
        let mut predictor = self.resource_predictor.lock();
        predictor.observe(task_key, sample);

        let completed = self.completed_tasks.fetch_add(1, Ordering::Relaxed) + 1;
        let _total_ms = self
            .total_task_time
            .fetch_add(duration.as_millis() as u64, Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();

        if elapsed > 0.0 {
            let throughput = completed as f64 / elapsed;
            tracing::debug!(
                task_key,
                duration_ms = duration.as_millis(),
                completed,
                throughput,
                "Task completed"
            );
        }
    }

    /// Recalculate optimal worker count based on current conditions
    pub fn reevaluate(&self) -> Option<usize> {
        let now = Instant::now();
        let last_adjustment = *self.last_adjustment.lock();

        if now.duration_since(last_adjustment) < self.config.reevaluation_interval {
            return None;
        }

        let memory_pressure = self.resource_governor.check_memory_pressure();
        let current = self.current_workers.load(Ordering::Relaxed);

        let new_workers = match memory_pressure {
            MemoryPressureLevel::Critical => {
                let throttled = self.resource_governor.optimal_parallelism(current);
                tracing::warn!(
                    current,
                    throttled,
                    "Throttling due to critical memory pressure"
                );
                throttled
            }
            MemoryPressureLevel::Warning => {
                let reduced = (current * 3 / 4).max(self.config.min_workers);
                tracing::info!(
                    current,
                    reduced,
                    "Reducing workers due to memory pressure warning"
                );
                reduced
            }
            MemoryPressureLevel::Normal => {
                let workload = *self.workload_type.read();
                self.calculate_optimal_for_workload(workload, current)
            }
        };

        let new_workers = new_workers.clamp(self.config.min_workers, self.config.max_workers);

        if new_workers != current {
            self.current_workers.store(new_workers, Ordering::Relaxed);
            *self.last_adjustment.lock() = now;
            tracing::info!(
                old = current,
                new = new_workers,
                memory_pressure = ?memory_pressure,
                "Adjusted worker count"
            );
            Some(new_workers)
        } else {
            None
        }
    }

    /// Calculate optimal workers based on workload type
    fn calculate_optimal_for_workload(&self, workload: WorkloadType, current: usize) -> usize {
        let cpu_count = num_cpus::get();

        match workload {
            WorkloadType::CpuBound => {
                cpu_count.clamp(self.config.min_workers, self.config.max_workers)
            }
            WorkloadType::IoBound => {
                let increased = (current * 5 / 4).min(self.config.max_workers);
                increased.max(self.config.base_workers)
            }
            WorkloadType::Mixed => self.config.base_workers,
            WorkloadType::Unknown => {
                if self.completed_tasks.load(Ordering::Relaxed) as usize
                    >= self.config.min_samples_for_classification
                {
                    self.classify_workload();
                    self.config.base_workers
                } else {
                    self.config.base_workers
                }
            }
        }
    }

    /// Classify workload type based on task patterns
    fn classify_workload(&self) {
        let total_tasks = self.completed_tasks.load(Ordering::Relaxed) as usize;
        if total_tasks < self.config.min_samples_for_classification {
            return;
        }

        let total_time_ms = self.total_task_time.load(Ordering::Relaxed);
        let avg_duration_ms = if total_tasks > 0 {
            total_time_ms / total_tasks as u64
        } else {
            0
        };

        let avg_duration = Duration::from_millis(avg_duration_ms);

        let cpu_bound_threshold = Duration::from_millis(500);
        let io_bound_threshold = Duration::from_millis(100);

        let new_type = if avg_duration > cpu_bound_threshold {
            WorkloadType::CpuBound
        } else if avg_duration < io_bound_threshold {
            WorkloadType::IoBound
        } else {
            WorkloadType::Mixed
        };

        let mut workload = self.workload_type.write();
        if *workload != new_type {
            tracing::info!(
                old = ?*workload,
                new = ?new_type,
                avg_duration_ms = avg_duration_ms,
                "Workload type classified"
            );
            *workload = new_type;
        }
    }

    /// Get current performance metrics
    #[allow(clippy::manual_checked_ops)]
    pub fn metrics(&self) -> PerformanceMetrics {
        let completed = self.completed_tasks.load(Ordering::Relaxed);
        let total_time_ms = self.total_task_time.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed();

        let avg_duration = if completed > 0 {
            Duration::from_millis(total_time_ms / completed)
        } else {
            Duration::ZERO
        };

        let throughput = if elapsed.as_secs_f64() > 0.0 {
            completed as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        let cpu_utilization = self.estimate_cpu_utilization();
        let memory_pressure = self.resource_governor.check_memory_pressure();
        let current_workers = self.current_workers.load(Ordering::Relaxed);
        let last_adjustment = *self.last_adjustment.lock();
        let time_since_adjustment = elapsed.saturating_sub(last_adjustment.elapsed());

        PerformanceMetrics {
            avg_task_duration: avg_duration,
            throughput,
            cpu_utilization,
            memory_pressure,
            current_workers,
            time_since_adjustment,
        }
    }

    /// Estimate CPU utilization based on task completion rate
    #[allow(clippy::manual_checked_ops)]
    fn estimate_cpu_utilization(&self) -> f64 {
        let current = self.current_workers.load(Ordering::Relaxed);
        let completed = self.completed_tasks.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();

        if elapsed == 0.0 || current == 0 {
            return 0.0;
        }

        let completion_rate = completed as f64 / elapsed;
        let max_completion_rate = current as f64 * 10.0;

        (completion_rate / max_completion_rate).min(1.0)
    }

    /// Get resource prediction for a task
    pub fn predict_resources(&self, task_key: &str) -> Option<Prediction> {
        let predictor = self.resource_predictor.lock();
        predictor.predict(task_key)
    }

    /// Check if task is warm (has enough historical data)
    pub fn is_task_warm(&self, task_key: &str) -> bool {
        let predictor = self.resource_predictor.lock();
        predictor.is_warm(task_key, self.config.min_samples_for_classification)
    }
}

impl Default for AdaptiveParallelismScheduler {
    fn default() -> Self {
        Self::new(AdaptiveConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_config_default() {
        let config = AdaptiveConfig::default();
        assert!(config.base_workers > 0);
        assert!(config.max_workers >= config.base_workers);
        assert!(config.min_workers >= 1);
        assert!(config.cpu_target > 0.0 && config.cpu_target <= 1.0);
    }

    #[test]
    fn test_adaptive_scheduler_creation() {
        let scheduler = AdaptiveParallelismScheduler::new(AdaptiveConfig::default());
        assert!(scheduler.optimal_workers() > 0);
    }

    #[test]
    fn test_task_recording() {
        let scheduler = AdaptiveParallelismScheduler::new(AdaptiveConfig::default());
        scheduler.record_task("test_task", Duration::from_millis(100), 1024 * 1024);
        let metrics = scheduler.metrics();
        assert!(metrics.throughput > 0.0);
    }

    #[test]
    fn test_workload_classification() {
        let scheduler = AdaptiveParallelismScheduler::new(AdaptiveConfig::default());

        for _ in 0..10 {
            scheduler.record_task("cpu_task", Duration::from_millis(600), 512 * 1024 * 1024);
        }

        scheduler.classify_workload();
        let workload = *scheduler.workload_type.read();
        assert_eq!(workload, WorkloadType::CpuBound);
    }

    #[test]
    fn test_io_bound_classification() {
        let scheduler = AdaptiveParallelismScheduler::new(AdaptiveConfig::default());

        for _ in 0..10 {
            scheduler.record_task("io_task", Duration::from_millis(50), 16 * 1024 * 1024);
        }

        scheduler.classify_workload();
        let workload = *scheduler.workload_type.read();
        assert_eq!(workload, WorkloadType::IoBound);
    }

    #[test]
    fn test_memory_pressure_throttling() {
        let config = AdaptiveConfig {
            base_workers: 8,
            max_workers: 16,
            min_workers: 1,
            memory_threshold: 50,
            ..Default::default()
        };
        let scheduler = AdaptiveParallelismScheduler::new(config);

        let governor = KernelResourceGovernor::new(None, Some(50));
        let current = scheduler.optimal_workers();
        let throttled = governor.optimal_parallelism(current);

        assert!(throttled <= current);
    }

    #[test]
    fn test_metrics_collection() {
        let scheduler = AdaptiveParallelismScheduler::new(AdaptiveConfig::default());

        scheduler.record_task("task1", Duration::from_millis(100), 1024 * 1024);
        scheduler.record_task("task2", Duration::from_millis(200), 2048 * 1024);

        let metrics = scheduler.metrics();
        assert!(metrics.avg_task_duration > Duration::ZERO);
        assert!(metrics.throughput >= 0.0);
        assert!(metrics.cpu_utilization >= 0.0 && metrics.cpu_utilization <= 1.0);
    }

    #[test]
    fn test_reevaluation_interval() {
        let config = AdaptiveConfig {
            reevaluation_interval: Duration::from_millis(100),
            ..Default::default()
        };
        let scheduler = AdaptiveParallelismScheduler::new(config);

        let result = scheduler.reevaluate();
        assert!(result.is_some());

        let result = scheduler.reevaluate();
        assert!(result.is_none());
    }
}
