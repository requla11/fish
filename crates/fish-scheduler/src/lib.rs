#![forbid(unsafe_code)]

pub mod adaptive;
pub mod bin_packing;
pub mod carbon;
pub mod jobserver_pool;
pub mod pipelining;
pub mod preemption;
pub mod racing;
pub mod resource_broker;
pub mod resource_governor;
pub mod resource_predictor;
pub mod watcher;
pub mod work_stealing;

pub use adaptive::{
    AdaptiveConfig, AdaptiveParallelismScheduler, PerformanceMetrics, WorkloadType,
};
pub use bin_packing::{AgentBucket, DteBinPacker, TaskTimingEstimate};
pub use jobserver_pool::JobserverPool;
pub use pipelining::{PipelineStage, PipelinedCompilationCoordinator};
pub use racing::DynamicRacingExecutor;
pub use resource_broker::{HostResourceBroker, HostResourceGuard};
pub use resource_governor::{KernelResourceGovernor, MemoryPressureLevel};
pub use watcher::FsWatcherDaemon;
pub use work_stealing::{ExecutionHeuristics, WorkStealingScheduler};

use std::cmp::Reverse;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use fish_executor::{Task, TaskExecutor, TaskOutcome, TaskStatus};
use fish_graph::{BuildGraph, GraphError, NodeId, TaskState};

#[derive(Debug, thiserror::Error)]
pub enum SchedulerError {
    #[error("invalid build graph: {0}")]
    InvalidGraph(#[from] GraphError),

    #[error("scheduler stalled with pending tasks; this is a bug, please report it")]
    Stalled,
}

#[derive(Debug, Clone)]
pub struct TaskTiming {
    pub label: String,
    pub description: String,
    pub start_offset: Duration,
    pub duration: Duration,
    pub node_id: NodeId,
    pub worker_id: usize,
    pub status: TaskStatus,
}

impl TaskTiming {
    pub fn new(label: impl Into<String>, duration: Duration, node_id: NodeId) -> Self {
        let label = label.into();
        Self {
            label: label.clone(),
            description: label,
            start_offset: Duration::ZERO,
            duration,
            node_id,
            worker_id: 0,
            status: TaskStatus::Executed,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FailureRecord {
    pub label: String,
    pub description: String,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone)]
pub struct BuildSummary {
    pub total: usize,
    pub executed: usize,
    pub cached: usize,
    pub failed: usize,
    pub cancelled: usize,
    pub duration: Duration,
    pub workers: usize,
    pub failures: Vec<FailureRecord>,
    pub timings: Vec<TaskTiming>,
}

impl BuildSummary {
    fn from_graph(
        graph: &BuildGraph<Task>,
        duration: Duration,
        workers: usize,
        failures: Vec<FailureRecord>,
        timings: Vec<TaskTiming>,
    ) -> Self {
        let mut executed = 0;
        let mut cached = 0;
        let mut failed = 0;
        let mut cancelled = 0;
        for node in graph.nodes() {
            match node.state {
                TaskState::Succeeded => executed += 1,
                TaskState::Cached => cached += 1,
                TaskState::Failed => failed += 1,
                TaskState::Cancelled => cancelled += 1,
                TaskState::Pending | TaskState::Running | TaskState::Ready | TaskState::Skipped => {
                }
            }
        }
        Self {
            total: graph.len(),
            executed,
            cached,
            failed,
            cancelled,
            duration,
            workers,
            failures,
            timings,
        }
    }

    pub fn succeeded(&self) -> bool {
        self.failed == 0
    }

    pub fn critical_path_report(
        &self,
        graph: &BuildGraph<Task>,
    ) -> Option<fish_graph::CriticalPathReport> {
        let mut durations = std::collections::HashMap::new();
        for timing in &self.timings {
            durations.insert(timing.node_id, timing.duration.as_millis() as u64);
        }
        fish_graph::CriticalPathAnalyzer::analyze(graph, &durations).ok()
    }

    pub fn to_chrome_trace(&self) -> serde_json::Value {
        let mut events = Vec::new();

        events.push(serde_json::json!({
            "name": "process_name",
            "ph": "M",
            "pid": 1,
            "args": {
                "name": "Fish Build Engine"
            }
        }));

        for w in 0..self.workers {
            events.push(serde_json::json!({
                "name": "thread_name",
                "ph": "M",
                "pid": 1,
                "tid": w,
                "args": {
                    "name": format!("Worker {w}")
                }
            }));
        }

        for timing in &self.timings {
            let status_str = match timing.status {
                TaskStatus::Executed => "executed",
                TaskStatus::Cached => "cached",
                TaskStatus::Failed => "failed",
            };
            events.push(serde_json::json!({
                "name": timing.label,
                "cat": "task",
                "ph": "X",
                "ts": timing.start_offset.as_micros(),
                "dur": timing.duration.as_micros(),
                "pid": 1,
                "tid": timing.worker_id,
                "args": {
                    "description": timing.description,
                    "status": status_str,
                    "duration_ms": timing.duration.as_secs_f64() * 1000.0,
                }
            }));
        }

        serde_json::json!({
            "traceEvents": events,
            "displayTimeUnit": "ms",
            "otherData": {
                "fish_version": env!("CARGO_PKG_VERSION"),
                "total_duration_ms": self.duration.as_secs_f64() * 1000.0,
                "total_tasks": self.total,
                "executed": self.executed,
                "cached": self.cached,
                "failed": self.failed,
            }
        })
    }

    pub fn write_chrome_trace(&self, path: &std::path::Path) -> std::io::Result<()> {
        let trace = self.to_chrome_trace();
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &trace)?;
        Ok(())
    }

    pub fn critical_path(&self, graph: &BuildGraph<Task>) -> (Duration, Vec<String>) {
        let n = graph.len();
        let mut timing_vec = vec![Duration::ZERO; n];
        for t in &self.timings {
            if t.node_id.index() < n {
                timing_vec[t.node_id.index()] = t.duration;
            }
        }

        let topo = graph.topological_order();
        let mut longest_to = vec![Duration::ZERO; n];
        let mut predecessor = vec![None; n];

        for &id in &topo {
            let idx = id.index();
            let own = timing_vec[idx];
            let (best_pred, best_cost) = graph
                .deps(id)
                .unwrap_or_default()
                .iter()
                .map(|dep| (Some(*dep), longest_to[dep.index()]))
                .max_by_key(|(_, cost)| *cost)
                .unwrap_or((None, Duration::ZERO));
            longest_to[idx] = best_cost + own;
            predecessor[idx] = best_pred;
        }

        let mut end = NodeId::from(0);
        let mut total = Duration::ZERO;
        for (idx, &cost) in longest_to.iter().enumerate() {
            if cost > total {
                total = cost;
                end = NodeId::from(idx);
            }
        }

        let mut path = Vec::new();
        let mut current = Some(end);
        while let Some(id) = current {
            if let Some(node) = graph.node(id) {
                path.push(node.payload.label.clone());
            }
            current = if id.index() < n {
                predecessor[id.index()]
            } else {
                None
            };
        }
        path.reverse();
        (total, path)
    }
}

type Outcome = (NodeId, TaskOutcome, usize, Duration);

#[allow(clippy::too_many_arguments)]
fn process_completion<G>(
    graph: &mut BuildGraph<Task>,
    in_flight: &mut usize,
    failures: &mut Vec<FailureRecord>,
    timings: &mut Vec<TaskTiming>,
    on_progress: &mut G,
    id: NodeId,
    outcome: TaskOutcome,
    worker_id: usize,
    start_offset: Duration,
) -> Result<(), SchedulerError>
where
    G: FnMut(&Task, &TaskOutcome),
{
    *in_flight -= 1;
    let (label, description) = match graph.node(id) {
        Some(node) => (node.payload.label.clone(), node.payload.description.clone()),
        None => return Err(SchedulerError::InvalidGraph(GraphError::MissingNode(id))),
    };
    timings.push(TaskTiming {
        label: label.clone(),
        description: description.clone(),
        start_offset,
        duration: outcome.duration,
        node_id: id,
        worker_id,
        status: outcome.status,
    });
    match outcome.status {
        TaskStatus::Executed => graph.set_state(id, TaskState::Succeeded)?,
        TaskStatus::Cached => graph.set_state(id, TaskState::Cached)?,
        TaskStatus::Failed => {
            graph.mark_failed(id)?;
            failures.push(FailureRecord {
                label,
                description,
                stdout: outcome.stdout.clone(),
                stderr: outcome.stderr.clone(),
            });
        }
    }
    if let Some(node) = graph.node(id) {
        on_progress(&node.payload, &outcome);
    }
    Ok(())
}

pub fn is_oom_failure(outcome: &TaskOutcome) -> bool {
    if let Some(code) = outcome.exit_code
        && (code == 137 || code == -1073741819 || code == (0xC0000005u32 as i32))
    {
        return true;
    }
    let lower = outcome.stderr.to_ascii_lowercase();
    lower.contains("out of memory")
        || lower.contains("virtual memory exhausted")
        || lower.contains("memory limit exceeded")
        || lower.contains("signal: 9")
        || lower.contains("killed: 9")
}

#[derive(Debug, Clone)]
pub struct Scheduler {
    workers: usize,
    critical_path: bool,
    ram_threshold: Option<u8>,
    ram_floor: usize,
    jobserver: Option<JobserverPool>,
    oom_retry: usize,
}

pub fn ram_capped_workers(
    available: u64,
    total: u64,
    requested: usize,
    floor: usize,
    threshold_percent: u8,
) -> usize {
    let threshold = threshold_percent.min(100) as u64;
    let constrained = total > 0 && available.saturating_mul(100) < total.saturating_mul(threshold);
    if constrained {
        floor.max(1).min(requested)
    } else {
        requested
    }
}

impl Scheduler {
    pub fn new(workers: usize) -> Self {
        Self {
            workers: workers.max(1),
            critical_path: true,
            ram_threshold: None,
            ram_floor: 1,
            jobserver: None,
            oom_retry: 1,
        }
    }

    pub fn with_oom_retry(mut self, max_retries: usize) -> Self {
        self.oom_retry = max_retries;
        self
    }

    pub fn with_jobserver(mut self, pool: JobserverPool) -> Self {
        self.jobserver = Some(pool);
        self
    }

    /// Enables (default) or disables critical-path-first task selection. When
    /// enabled, ready tasks whose dependency tail is the longest are picked
    /// first, which minimizes idle worker gaps on wide build graphs.
    pub fn with_critical_path_priority(mut self, enabled: bool) -> Self {
        self.critical_path = enabled;
        self
    }

    /// Throttles the build to `floor` concurrent workers whenever the system's
    /// available memory drops below `threshold_percent` of the total memory.
    pub fn with_ram_backpressure(mut self, threshold_percent: u8, floor: usize) -> Self {
        self.ram_threshold = Some(threshold_percent.min(100));
        self.ram_floor = floor.max(1);
        self
    }

    pub fn run<E, G>(
        &self,
        graph: &mut BuildGraph<Task>,
        executor: &E,
        mut on_progress: G,
    ) -> Result<BuildSummary, SchedulerError>
    where
        E: TaskExecutor,
        G: FnMut(&Task, &TaskOutcome),
    {
        tracing::info!(
            workers = self.workers,
            total_tasks = graph.len(),
            "Starting build execution"
        );

        graph.validate()?;
        graph.reset_states();
        let tail = critical_path_tails(graph);
        let start = Instant::now();
        let (tx, rx): (mpsc::Sender<Outcome>, mpsc::Receiver<Outcome>) = mpsc::channel();
        let mut in_flight: usize = 0;
        let mut failures: Vec<FailureRecord> = Vec::new();
        let mut timings: Vec<TaskTiming> = Vec::new();
        let mut free_workers: Vec<usize> = (0..self.workers).rev().collect();
        let mut last_ram_check = Instant::now() - Duration::from_secs(1);
        let mut ram_limited = false;
        let mut sys = sysinfo::System::new();
        sys.refresh_memory();
        let mut oom_retried: std::collections::HashMap<NodeId, usize> =
            std::collections::HashMap::new();

        let result = thread::scope(|scope| -> Result<(), SchedulerError> {
            let (task_tx, task_rx) =
                crossbeam_channel::unbounded::<(NodeId, Task, usize, Duration)>();
            let jobserver_pool = self.jobserver.clone();
            for _ in 0..self.workers {
                let task_rx = task_rx.clone();
                let tx = tx.clone();
                let js_pool = jobserver_pool.clone();
                scope.spawn(move || {
                    while let Ok((id, task, worker_id, task_start_offset)) = task_rx.recv() {
                        let _token = js_pool.as_ref().and_then(|p| p.acquire().ok());
                        let outcome =
                            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                executor.execute(&task)
                            })) {
                                Ok(Ok(outcome)) => outcome,
                                Ok(Err(error)) => TaskOutcome::failed(&task, error.to_string()),
                                Err(panic) => {
                                    let message = panic
                                        .downcast_ref::<&str>()
                                        .map(|s| s.to_string())
                                        .or_else(|| panic.downcast_ref::<String>().cloned())
                                        .unwrap_or_else(|| "unknown panic".to_string());
                                    TaskOutcome::failed(
                                        &task,
                                        format!("executor panicked: {message}"),
                                    )
                                }
                            };
                        drop(_token);
                        let _ = tx.send((id, outcome, worker_id, task_start_offset));
                    }
                });
            }

            let mut ready: Vec<NodeId> = graph.ready_nodes();
            if self.critical_path {
                ready.sort_by_key(|id| Reverse(tail[id.index()]));
            }
            let mut enqueued = vec![false; graph.len()];
            for &id in &ready {
                enqueued[id.index()] = true;
            }
            let mut ready_index: usize = 0;
            loop {
                let effective_workers = if let Some(threshold) = self.ram_threshold {
                    if last_ram_check.elapsed() >= Duration::from_millis(500) {
                        last_ram_check = Instant::now();
                        sys.refresh_memory();
                        ram_limited = sys.available_memory().saturating_mul(100)
                            < sys.total_memory().saturating_mul(threshold as u64);
                    }
                    if ram_limited {
                        self.ram_floor.min(self.workers)
                    } else {
                        self.workers
                    }
                } else {
                    self.workers
                };
                while in_flight < effective_workers && ready_index < ready.len() {
                    let id = ready[ready_index];
                    ready_index += 1;
                    graph.set_state(id, TaskState::Running)?;
                    let task = graph.node(id).expect("ready nodes exist").payload.clone();
                    in_flight += 1;
                    let worker_id = free_workers.pop().unwrap_or(0);
                    let task_start_offset = start.elapsed();

                    tracing::debug!(
                        task_id = id.index(),
                        task_label = %task.label,
                        worker_id,
                        in_flight,
                        "Dispatching task to worker"
                    );

                    let _ = task_tx.send((id, task, worker_id, task_start_offset));
                }

                if ready_index >= ready.len() && in_flight == 0 {
                    if graph.nodes().iter().all(|node| node.state.is_terminal()) {
                        break;
                    }
                    return Err(SchedulerError::Stalled);
                }

                if in_flight == 0 {
                    return Err(SchedulerError::Stalled);
                }

                let (id, outcome, worker_id, task_start_offset) =
                    rx.recv().map_err(|_| SchedulerError::Stalled)?;
                free_workers.push(worker_id);

                tracing::debug!(
                    task_id = id.index(),
                    status = ?outcome.status,
                    duration_ms = outcome.duration.as_millis(),
                    worker_id,
                    "Task completed"
                );

                if outcome.status == TaskStatus::Failed && is_oom_failure(&outcome) {
                    let retries = oom_retried.entry(id).or_insert(0);
                    if *retries < self.oom_retry {
                        *retries += 1;
                        in_flight -= 1;
                        let _ = graph.set_state(id, TaskState::Pending);
                        if let Some(node) = graph.node_mut(id) {
                            node.payload.resources.exclusive = true;
                            node.payload.resources.permits =
                                (node.payload.resources.permits * 2).max(2);
                        }
                        ready.push(id);
                        continue;
                    }
                }

                let was_success = outcome.status != TaskStatus::Failed;
                process_completion(
                    graph,
                    &mut in_flight,
                    &mut failures,
                    &mut timings,
                    &mut on_progress,
                    id,
                    outcome,
                    worker_id,
                    task_start_offset,
                )?;
                if was_success && let Ok(dependents) = graph.dependents(id) {
                    for &dep in dependents {
                        let dep_idx = dep.index();
                        if !enqueued[dep_idx]
                            && matches!(graph.state(dep), Ok(TaskState::Pending))
                            && graph.is_ready(dep).unwrap_or(false)
                        {
                            enqueued[dep_idx] = true;
                            if self.critical_path {
                                let tail_val = tail[dep_idx];
                                let slice = &ready[ready_index..];
                                let pos = slice
                                    .binary_search_by(|probe| {
                                        tail[probe.index()].cmp(&tail_val).reverse()
                                    })
                                    .unwrap_or_else(|e| e);
                                ready.insert(ready_index + pos, dep);
                            } else {
                                ready.push(dep);
                            }
                        }
                    }
                }
                while let Ok((id, outcome, worker_id, task_start_offset)) = rx.try_recv() {
                    free_workers.push(worker_id);

                    tracing::debug!(
                        task_id = id.index(),
                        status = ?outcome.status,
                        duration_ms = outcome.duration.as_millis(),
                        worker_id,
                        "Task completed (batch)"
                    );

                    let was_success_batch = outcome.status != TaskStatus::Failed;
                    process_completion(
                        graph,
                        &mut in_flight,
                        &mut failures,
                        &mut timings,
                        &mut on_progress,
                        id,
                        outcome,
                        worker_id,
                        task_start_offset,
                    )?;
                    if was_success_batch && let Ok(dependents) = graph.dependents(id) {
                        for &dep in dependents {
                            let dep_idx = dep.index();
                            if !enqueued[dep_idx]
                                && matches!(graph.state(dep), Ok(TaskState::Pending))
                                && graph.is_ready(dep).unwrap_or(false)
                            {
                                enqueued[dep_idx] = true;
                                if self.critical_path {
                                    let tail_val = tail[dep_idx];
                                    let slice = &ready[ready_index..];
                                    let pos = slice
                                        .binary_search_by(|probe| {
                                            tail[probe.index()].cmp(&tail_val).reverse()
                                        })
                                        .unwrap_or_else(|e| e);
                                    ready.insert(ready_index + pos, dep);
                                } else {
                                    ready.push(dep);
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        });

        result?;

        let summary =
            BuildSummary::from_graph(graph, start.elapsed(), self.workers, failures, timings);

        tracing::info!(
            total = summary.total,
            executed = summary.executed,
            cached = summary.cached,
            failed = summary.failed,
            duration_sec = summary.duration.as_secs_f64(),
            "Build execution completed"
        );

        Ok(summary)
    }
}

/// Longest path (in edges) from each node down to a leaf of the dependency
/// DAG, measured through `dependents` (reverse topological order); tasks with
/// heavier tails are scheduled first.
fn critical_path_tails(graph: &BuildGraph<Task>) -> Vec<usize> {
    let mut tail = vec![0usize; graph.len()];
    for id in graph.topological_order().into_iter().rev() {
        let longest_dependent = graph
            .dependents(id)
            .unwrap_or_default()
            .iter()
            .map(|dep| tail[dep.index()])
            .max()
            .unwrap_or(0);
        tail[id.index()] = longest_dependent + 1;
    }
    tail
}

#[cfg(test)]
mod tests {
    use super::*;
    use fish_executor::{CommandSpec, ExecutorError, ProcessExecutor, Task};
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn chain_graph(labels: &[&str]) -> BuildGraph<Task> {
        let mut graph = BuildGraph::new();
        let mut ids = Vec::new();
        for label in labels {
            let spec = CommandSpec::new("echo").arg(*label);
            ids.push(graph.add_node(Task::new(String::from(*label), spec.command_line(), spec)));
        }
        for window in ids.windows(2) {
            graph
                .add_dependency(window[0], window[1])
                .expect("chain edges are acyclic");
        }
        graph
    }

    fn fanout_graph(labels: &[&str]) -> BuildGraph<Task> {
        let mut graph = BuildGraph::new();
        let mut ids = Vec::new();
        for label in labels {
            let spec = CommandSpec::new("echo").arg(*label);
            ids.push(graph.add_node(Task::new(String::from(*label), spec.command_line(), spec)));
        }
        for id in ids.iter().skip(1) {
            graph
                .add_dependency(ids[0], *id)
                .expect("fanout edges are acyclic");
        }
        graph
    }

    #[test]
    fn critical_path_reports_the_longest_dependency_chain() {
        let mut graph = BuildGraph::new();
        let a = graph.add_node(Task::new(
            "a".to_string(),
            "a".to_string(),
            CommandSpec::new("echo").arg("a"),
        ));
        let b = graph.add_node(Task::new(
            "b".to_string(),
            "b".to_string(),
            CommandSpec::new("echo").arg("b"),
        ));
        let c = graph.add_node(Task::new(
            "c".to_string(),
            "c".to_string(),
            CommandSpec::new("echo").arg("c"),
        ));
        graph.add_dependency(a, b).expect("edge a -> b");
        graph.add_dependency(a, c).expect("edge a -> c");
        graph.add_dependency(b, c).expect("edge b -> c");

        let timings = vec![
            TaskTiming::new("a", Duration::from_secs(1), a),
            TaskTiming::new("b", Duration::from_secs(3), b),
            TaskTiming::new("c", Duration::from_secs(1), c),
        ];
        let summary = BuildSummary::from_graph(&graph, Duration::ZERO, 2, vec![], timings);

        let (total, path) = summary.critical_path(&graph);
        assert_eq!(
            total,
            Duration::from_secs(5),
            "a + b + c dominates the diamond"
        );
        assert_eq!(path, vec!["a", "b", "c"]);
    }

    #[test]
    fn critical_path_is_deterministic_on_ties() {
        let mut graph = BuildGraph::new();
        let a = graph.add_node(Task::new(
            "a".to_string(),
            "a".to_string(),
            CommandSpec::new("echo").arg("a"),
        ));
        let b = graph.add_node(Task::new(
            "b".to_string(),
            "b".to_string(),
            CommandSpec::new("echo").arg("b"),
        ));
        let timings = vec![
            TaskTiming::new("a", Duration::from_secs(1), a),
            TaskTiming::new("b", Duration::from_secs(1), b),
        ];
        let summary = BuildSummary::from_graph(&graph, Duration::ZERO, 2, vec![], timings);

        let expected = summary.critical_path(&graph).1;
        for _ in 0..20 {
            assert_eq!(summary.critical_path(&graph).1, expected);
        }
        assert_eq!(expected, vec!["a"]);
    }

    #[derive(Default)]
    struct FakeExecutor {
        order: Mutex<Vec<String>>,
        max_concurrent: AtomicUsize,
        concurrent: AtomicUsize,
        delays: HashMap<String, Duration>,
        fail: Vec<String>,
        cached: Vec<String>,
        spawn_errors: Vec<String>,
    }

    impl FakeExecutor {
        fn with_delay(mut self, label: &str, delay: Duration) -> Self {
            self.delays.insert(label.to_string(), delay);
            self
        }

        fn failing(mut self, labels: &[&str]) -> Self {
            self.fail = labels.iter().map(|s| s.to_string()).collect();
            self
        }

        fn with_cached(mut self, labels: &[&str]) -> Self {
            self.cached = labels.iter().map(|s| s.to_string()).collect();
            self
        }

        fn with_spawn_error(mut self, labels: &[&str]) -> Self {
            self.spawn_errors = labels.iter().map(|s| s.to_string()).collect();
            self
        }
    }

    impl TaskExecutor for FakeExecutor {
        fn execute(&self, task: &Task) -> Result<TaskOutcome, ExecutorError> {
            if self.cached.contains(&task.label) {
                return Ok(TaskOutcome::cached(task));
            }
            if self.spawn_errors.contains(&task.label) {
                return Err(ExecutorError::Spawn {
                    command: task.label.clone(),
                    source: std::io::Error::other("simulated spawn failure"),
                });
            }
            let current = self.concurrent.fetch_add(1, Ordering::SeqCst) + 1;
            self.max_concurrent.fetch_max(current, Ordering::SeqCst);
            if let Some(delay) = self.delays.get(&task.label) {
                thread::sleep(*delay);
            }
            let outcome = if self.fail.contains(&task.label) {
                TaskOutcome::failed(task, "boom")
            } else {
                TaskOutcome::executed(task)
            };
            self.order.lock().unwrap().push(task.label.clone());
            self.concurrent.fetch_sub(1, Ordering::SeqCst);
            Ok(outcome)
        }
    }

    #[test]
    fn executes_a_chain_in_order() {
        let mut graph = chain_graph(&["a", "b", "c"]);
        let executor = FakeExecutor::default();
        let scheduler = Scheduler::new(4);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.executed, 3);
        assert_eq!(summary.failed, 0);
        assert!(summary.succeeded());
        assert_eq!(executor.order.lock().unwrap().as_slice(), ["a", "b", "c"]);
    }

    #[test]
    fn runs_independent_tasks_in_parallel() {
        let mut graph = fanout_graph(&["root", "leaf1", "leaf2"]);
        let executor = FakeExecutor::default()
            .with_delay("leaf1", Duration::from_millis(50))
            .with_delay("leaf2", Duration::from_millis(50));
        let scheduler = Scheduler::new(4);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
        assert_eq!(summary.executed, 3);
        assert!(summary.succeeded());

        assert_eq!(executor.max_concurrent.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn respects_the_worker_limit() {
        let mut graph = fanout_graph(&["root", "l1", "l2", "l3", "l4"]);
        let executor = FakeExecutor::default()
            .with_delay("l1", Duration::from_millis(20))
            .with_delay("l2", Duration::from_millis(20))
            .with_delay("l3", Duration::from_millis(20))
            .with_delay("l4", Duration::from_millis(20));
        let scheduler = Scheduler::new(2);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
        assert_eq!(summary.executed, 5);
        assert_eq!(executor.max_concurrent.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_failure_cancels_dependents() {
        let mut graph = chain_graph(&["a", "b", "c"]);
        let executor = FakeExecutor::default().failing(&["b"]);
        let scheduler = Scheduler::new(2);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
        assert_eq!(summary.executed, 1);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.cancelled, 1);
        assert!(!summary.succeeded());
        assert_eq!(summary.failures.len(), 1);
        assert_eq!(summary.failures[0].label, "b");
        assert_eq!(executor.order.lock().unwrap().as_slice(), ["a", "b"]);
    }

    #[test]
    fn cached_tasks_are_counted() {
        let mut graph = chain_graph(&["a", "b"]);
        let executor = FakeExecutor::default().with_cached(&["b"]);
        let scheduler = Scheduler::new(2);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
        assert_eq!(summary.cached, 1);
        assert_eq!(summary.executed, 1);
        assert!(summary.succeeded());
        assert_eq!(executor.order.lock().unwrap().as_slice(), ["a"]);
    }

    #[test]
    fn reports_hermetic_spawn_errors_as_failures() {
        let mut graph = chain_graph(&["a", "b"]);
        let executor = FakeExecutor::default().with_spawn_error(&["a"]);
        let scheduler = Scheduler::new(1);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.cancelled, 1);
        assert!(!summary.succeeded());
        assert!(
            summary.failures[0]
                .stderr
                .contains("simulated spawn failure")
        );
    }

    #[test]
    fn reports_executor_spawn_errors_as_failures() {
        let executor = ProcessExecutor::new(false);
        let spec = CommandSpec::new("fish-definitely-not-a-program-xyz");
        let task = Task::new("missing-program", spec.command_line(), spec);
        let mut graph = BuildGraph::new();
        graph.add_node(task);
        let scheduler = Scheduler::new(1);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
        assert_eq!(summary.failed, 1);
        assert!(summary.failures[0].stderr.contains("failed to spawn"));
    }

    #[test]
    fn reports_failed_command_exit_codes() {
        let executor = ProcessExecutor::new(false);
        let spec = CommandSpec::new("cargo")
            .arg("metadata")
            .arg("--manifest-path")
            .arg("definitely-missing-path/Cargo.toml");
        let task = Task::new("failing-cargo", spec.command_line(), spec);
        let mut graph = BuildGraph::new();
        graph.add_node(task);
        let scheduler = Scheduler::new(1);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.failures[0].label, "failing-cargo");
        assert!(!summary.failures[0].stderr.is_empty());
    }

    #[test]
    fn worker_count_defaults_to_at_least_one() {
        assert_eq!(Scheduler::new(0).workers, 1);
        assert_eq!(Scheduler::new(1).workers, 1);
        assert_eq!(Scheduler::new(4).workers, 4);
    }

    #[test]
    fn critical_path_priority_schedules_heavy_chains_first() {
        let mut graph = BuildGraph::new();
        let _b = graph.add_node(Task::new(
            "b".to_string(),
            "b".to_string(),
            CommandSpec::new("echo").arg("b"),
        ));
        let a0 = graph.add_node(Task::new(
            "a0".to_string(),
            "a0".to_string(),
            CommandSpec::new("echo").arg("a0"),
        ));
        let a1 = graph.add_node(Task::new(
            "a1".to_string(),
            "a1".to_string(),
            CommandSpec::new("echo").arg("a1"),
        ));
        graph.add_dependency(a0, a1).expect("a0 -> a1");

        let executor = FakeExecutor::default();
        let scheduler = Scheduler::new(1);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
        assert!(summary.succeeded());

        let order = executor.order.lock().unwrap().clone();
        assert_eq!(
            order.first(),
            Some(&"a0".to_string()),
            "the node whose tail is longer (a0 -> a1) must be picked before the independent b"
        );
        let a0_pos = order.iter().position(|l| l == "a0").unwrap();
        let a1_pos = order.iter().position(|l| l == "a1").unwrap();
        assert!(a0_pos < a1_pos, "a1 cannot run before its dependency a0");
    }

    #[test]
    fn without_priority_the_node_order_wins() {
        let mut graph = BuildGraph::new();
        let _b = graph.add_node(Task::new(
            "b".to_string(),
            "b".to_string(),
            CommandSpec::new("echo").arg("b"),
        ));
        let a0 = graph.add_node(Task::new(
            "a0".to_string(),
            "a0".to_string(),
            CommandSpec::new("echo").arg("a0"),
        ));
        let a1 = graph.add_node(Task::new(
            "a1".to_string(),
            "a1".to_string(),
            CommandSpec::new("echo").arg("a1"),
        ));
        graph.add_dependency(a0, a1).expect("a0 -> a1");

        let executor = FakeExecutor::default();
        let scheduler = Scheduler::new(1).with_critical_path_priority(false);
        scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();

        let order = executor.order.lock().unwrap().clone();
        assert_eq!(order, vec!["b", "a0", "a1"]);
    }

    #[test]
    fn ram_capped_workers_only_throttles_below_the_threshold() {
        assert_eq!(ram_capped_workers(8000, 10000, 8, 1, 20), 8);
        assert_eq!(ram_capped_workers(1000, 10000, 8, 1, 20), 1);
        assert_eq!(ram_capped_workers(0, 10000, 8, 2, 10), 2);
        assert_eq!(
            ram_capped_workers(1000, 0, 8, 2, 10),
            8,
            "unknown totals never throttle"
        );
        assert_eq!(
            ram_capped_workers(1, 100, 4, 1, 100),
            1,
            "100% free always throttles"
        );
        assert_eq!(
            ram_capped_workers(1, 100, 4, 4, 100),
            4,
            "floor is capped by requested"
        );
        assert_eq!(ram_capped_workers(99, 100, 4, 2, 100), 2);
    }

    #[test]
    fn ram_backpressure_still_completes_a_build() {
        let mut graph = chain_graph(&["a", "b", "c"]);
        let executor = FakeExecutor::default();
        let scheduler = Scheduler::new(4).with_ram_backpressure(1, 1);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
        assert!(summary.succeeded());
        assert_eq!(summary.executed, 3);
    }

    #[test]
    fn test_chrome_trace_generation() {
        let mut graph = chain_graph(&["task1", "task2"]);
        let executor = FakeExecutor::default();
        let scheduler = Scheduler::new(2);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();

        let trace = summary.to_chrome_trace();
        let events = trace["traceEvents"].as_array().unwrap();
        assert!(events.iter().any(|e| e["name"] == "process_name"));
        assert!(
            events
                .iter()
                .any(|e| e["name"] == "thread_name" && e["args"]["name"] == "Worker 0")
        );
        assert!(events.iter().any(|e| e["name"] == "task1"));
        assert!(events.iter().any(|e| e["name"] == "task2"));
    }

    #[test]
    fn test_is_oom_failure() {
        let mut outcome = TaskOutcome::executed(&Task::new(
            "t",
            "d",
            fish_executor::CommandSpec::new("echo"),
        ));
        assert!(!is_oom_failure(&outcome));

        outcome.status = TaskStatus::Failed;
        outcome.exit_code = Some(137);
        assert!(is_oom_failure(&outcome));

        outcome.exit_code = Some(1);
        outcome.stderr = "error: out of memory allocating 4GB".to_string();
        assert!(is_oom_failure(&outcome));

        outcome.stderr = "compilation error: undefined reference".to_string();
        assert!(!is_oom_failure(&outcome));
    }

    #[test]
    fn test_oom_retry_recovers() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        struct OomOnceExecutor {
            attempts: AtomicUsize,
        }

        impl TaskExecutor for OomOnceExecutor {
            fn execute(&self, task: &Task) -> Result<TaskOutcome, fish_executor::ExecutorError> {
                let prev = self.attempts.fetch_add(1, Ordering::SeqCst);
                if prev == 0 {
                    let mut outcome = TaskOutcome::failed(task, "fatal error: out of memory");
                    outcome.exit_code = Some(137);
                    Ok(outcome)
                } else {
                    Ok(TaskOutcome::executed(task))
                }
            }
        }

        let mut graph = chain_graph(&["heavy_task"]);
        let executor = OomOnceExecutor {
            attempts: AtomicUsize::new(0),
        };
        let scheduler = Scheduler::new(2).with_oom_retry(1);
        let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();

        assert!(summary.succeeded());
        assert_eq!(summary.executed, 1);
        assert_eq!(executor.attempts.load(Ordering::SeqCst), 2);
    }
}
