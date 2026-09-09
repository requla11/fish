#![forbid(unsafe_code)]

use fish_executor::{Task, TaskExecutor, TaskOutcome, TaskStatus};
use fish_graph::{BuildGraph, NodeId, TaskState};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::{BuildSummary, FailureRecord, SchedulerError, TaskTiming};

#[derive(Default, Clone)]
pub struct ExecutionHeuristics {
    pub historical_durations: dashmap::DashMap<String, Duration>,
}

impl ExecutionHeuristics {
    pub fn get_estimated_weight(&self, task: &Task) -> u64 {
        if let Some(avg_duration) = self.historical_durations.get(&task.label) {
            avg_duration.as_millis() as u64
        } else {
            100
        }
    }

    pub fn record_execution(&self, task: &Task, duration: Duration) {
        if let Some(mut existing) = self.historical_durations.get_mut(&task.label) {
            let new_avg = (*existing * 8 + duration * 2) / 10;
            *existing = new_avg;
        } else {
            self.historical_durations
                .insert(task.label.clone(), duration);
        }
    }
}

pub struct WorkStealingScheduler {
    worker_count: usize,
    graph: BuildGraph<Task>,
    executor: Arc<dyn TaskExecutor>,
    heuristics: Arc<ExecutionHeuristics>,
}

impl WorkStealingScheduler {
    pub fn new(
        worker_count: usize,
        graph: BuildGraph<Task>,
        executor: Arc<dyn TaskExecutor>,
    ) -> Self {
        let worker_count = worker_count.max(1);

        Self {
            worker_count,
            graph,
            executor,
            heuristics: Arc::new(ExecutionHeuristics::default()),
        }
    }

    pub fn with_heuristics(mut self, heuristics: Arc<ExecutionHeuristics>) -> Self {
        self.heuristics = heuristics;
        self
    }

    fn compute_all_tail_lengths(&self) -> Vec<usize> {
        let n = self.graph.len();
        let mut tail_lengths = vec![0; n];
        let topo = self.graph.topological_order();
        for &id in topo.iter().rev() {
            let max_child = self
                .graph
                .dependents(id)
                .unwrap_or_default()
                .iter()
                .map(|child| tail_lengths[child.index()] + 1)
                .max()
                .unwrap_or(0);
            tail_lengths[id.index()] = max_child;
        }
        tail_lengths
    }

    fn priority_score(&self, id: NodeId, tail_lengths: &[usize]) -> u64 {
        let tail_length = tail_lengths.get(id.index()).copied().unwrap_or(0) as u64;
        let task = &self.graph.node(id).expect("ready nodes exist").payload;
        let weight = self.heuristics.get_estimated_weight(task);
        tail_length * 1000 + weight
    }

    pub fn run(&mut self) -> Result<BuildSummary, SchedulerError> {
        let start = Instant::now();

        self.graph.validate()?;
        self.graph.reset_states();

        let injector = Arc::new(crossbeam_deque::Injector::new());
        let (done_tx, done_rx) =
            crossbeam_channel::unbounded::<(NodeId, TaskOutcome, Duration, usize)>();
        let (sleep_tx, sleep_rx) = crossbeam_channel::unbounded::<()>();

        let mut workers_local = Vec::with_capacity(self.worker_count);
        let mut stealers = Vec::with_capacity(self.worker_count);

        for _ in 0..self.worker_count {
            let worker = crossbeam_deque::Worker::new_fifo();
            stealers.push(worker.stealer());
            workers_local.push(worker);
        }
        let stealers = Arc::new(stealers);

        let mut workers = Vec::with_capacity(self.worker_count);
        for worker_id in 0..self.worker_count {
            let done_tx = done_tx.clone();
            let sleep_rx = sleep_rx.clone();
            let executor = Arc::clone(&self.executor);
            let heuristics = Arc::clone(&self.heuristics);
            let build_start = start;
            let local_deque = workers_local.pop().unwrap();
            let global_injector = Arc::clone(&injector);
            let sibling_stealers = Arc::clone(&stealers);
            
            workers.push(std::thread::spawn(move || {
                let mut rng_seed = worker_id as u64;
                let num_siblings = sibling_stealers.len();
                
                loop {
                    // 1. Try local deque
                    let mut task_opt = local_deque.pop();

                    // 2. Try global injector
                    if task_opt.is_none() {
                        loop {
                            match global_injector.steal_batch_and_pop(&local_deque) {
                                crossbeam_deque::Steal::Success(t) => { task_opt = Some(t); break; }
                                crossbeam_deque::Steal::Empty => break,
                                crossbeam_deque::Steal::Retry => continue,
                            }
                        }
                    }

                    // 3. Try stealing from siblings
                    if task_opt.is_none() && num_siblings > 1 {
                        rng_seed = rng_seed.wrapping_add(1);
                        let start_idx = (rng_seed % num_siblings as u64) as usize;
                        for i in 0..num_siblings {
                            let target = (start_idx + i) % num_siblings;
                            if target == worker_id { continue; }
                            loop {
                                match sibling_stealers[target].steal_batch_and_pop(&local_deque) {
                                    crossbeam_deque::Steal::Success(t) => { task_opt = Some(t); break; }
                                    crossbeam_deque::Steal::Empty => break,
                                    crossbeam_deque::Steal::Retry => continue,
                                }
                            }
                            if task_opt.is_some() { break; }
                        }
                    }

                    if let Some((id, task)) = task_opt {
                        let task_start = Instant::now();
                        let start_offset = task_start.saturating_duration_since(build_start);
                        let outcome = match std::panic::catch_unwind(AssertUnwindSafe(|| {
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
                                TaskOutcome::failed(&task, format!("executor panicked: {message}"))
                            }
                        };
                        heuristics.record_execution(&task, task_start.elapsed());
                        let _ = done_tx.send((id, outcome, start_offset, worker_id));
                    } else {
                        // Sleep until woken up by the orchestrator or shutdown
                        if sleep_rx.recv().is_err() {
                            break;
                        }
                    }
                }
            }));
        }

        let mut in_flight: usize = 0;
        let mut failures: Vec<FailureRecord> = Vec::new();
        let mut timings: Vec<TaskTiming> = Vec::new();
        let tail_lengths = self.compute_all_tail_lengths();

        // O(1) scheduling: track indegrees locally
        let mut indegrees = vec![0; self.graph.len()];
        let mut ready = Vec::new();

        for id in self.graph.topological_order() {
            let deps = self.graph.deps(id).unwrap_or_default();
            indegrees[id.index()] = deps.len();
            if deps.is_empty() {
                ready.push(id);
            }
        }

        loop {
            if !ready.is_empty() {
                ready.sort_unstable_by_key(|id| {
                    (
                        std::cmp::Reverse(self.priority_score(*id, &tail_lengths)),
                        id.index(),
                    )
                });
                for id in ready.drain(..) {
                    let task = self
                        .graph
                        .node(id)
                        .expect("ready nodes exist")
                        .payload
                        .clone();
                    self.graph.set_state(id, TaskState::Running)?;
                    injector.push((id, task));
                    in_flight += 1;
                    let _ = sleep_tx.send(());
                }
            }

            if in_flight == 0 {
                break;
            }

            let (id, outcome, start_offset, worker_id) =
                done_rx.recv().map_err(|_| SchedulerError::Stalled)?;
            in_flight -= 1;
            
            let is_success = outcome.status == TaskStatus::Executed || outcome.status == TaskStatus::Cached;
            self.apply_outcome(
                id,
                outcome,
                start_offset,
                worker_id,
                &mut failures,
                &mut timings,
            )?;
            
            if is_success {
                for &dependent in self.graph.dependents(id).unwrap_or_default() {
                    indegrees[dependent.index()] -= 1;
                    if indegrees[dependent.index()] == 0 {
                        ready.push(dependent);
                    }
                }
            }
            
            while let Ok((id, outcome, start_offset, worker_id)) = done_rx.try_recv() {
                in_flight -= 1;
                let is_success = outcome.status == TaskStatus::Executed || outcome.status == TaskStatus::Cached;
                self.apply_outcome(
                    id,
                    outcome,
                    start_offset,
                    worker_id,
                    &mut failures,
                    &mut timings,
                )?;
                if is_success {
                    for &dependent in self.graph.dependents(id).unwrap_or_default() {
                        indegrees[dependent.index()] -= 1;
                        if indegrees[dependent.index()] == 0 {
                            ready.push(dependent);
                        }
                    }
                }
            }
        }

        drop(sleep_tx);
        for worker in workers {
            let _ = worker.join();
        }

        Ok(BuildSummary::from_graph(
            &self.graph,
            start.elapsed(),
            self.worker_count,
            failures,
            timings,
        ))
    }

    fn apply_outcome(
        &mut self,
        id: NodeId,
        outcome: TaskOutcome,
        start_offset: Duration,
        worker_id: usize,
        failures: &mut Vec<FailureRecord>,
        timings: &mut Vec<TaskTiming>,
    ) -> Result<(), SchedulerError> {
        if self.graph.state(id)? == TaskState::Cancelled {
            return Ok(());
        }

        let task = self.graph.node(id).map(|node| node.payload.clone());
        if let Some(task) = &task {
            timings.push(TaskTiming {
                label: task.label.clone(),
                description: task.description.clone(),
                start_offset,
                duration: outcome.duration,
                node_id: id,
                worker_id,
                status: outcome.status,
            });
        }

        match outcome.status {
            TaskStatus::Executed => self.graph.set_state(id, TaskState::Succeeded)?,
            TaskStatus::Cached => self.graph.set_state(id, TaskState::Cached)?,
            TaskStatus::Failed => {
                self.graph.mark_failed(id)?;
                if let Some(task) = &task {
                    failures.push(FailureRecord {
                        label: task.label.clone(),
                        description: task.description.clone(),
                        stdout: outcome.stdout.clone(),
                        stderr: outcome.stderr.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fish_executor::{CommandSpec, ExecutorError};
    use std::collections::HashSet;
    use std::sync::Arc;

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

    struct SelectiveExecutor {
        fail: HashSet<String>,
    }

    impl TaskExecutor for SelectiveExecutor {
        fn execute(&self, task: &Task) -> Result<TaskOutcome, ExecutorError> {
            if self.fail.contains(&task.label) {
                Ok(TaskOutcome::failed(task, "boom"))
            } else {
                Ok(TaskOutcome::executed(task))
            }
        }
    }

    #[test]
    fn test_ml_work_stealing_distribution() {
        let mut graph = fish_graph::BuildGraph::new();

        for i in 0..10 {
            let spec = CommandSpec::new("echo").arg(format!("task_{}", i));
            let task = Task::new(format!("task_{}", i), spec.command_line(), spec);
            graph.add_node(task);
        }

        let executor = Arc::new(SelectiveExecutor {
            fail: HashSet::new(),
        });
        let mut scheduler = WorkStealingScheduler::new(4, graph, executor);

        let result = scheduler.run();
        assert!(result.is_ok());

        let summary = result.unwrap();
        assert_eq!(summary.total, 10);
        assert_eq!(summary.executed, 10);
    }

    #[test]
    fn work_stealing_runs_transitive_dependencies() {
        let graph = chain_graph(&["a", "b", "c"]);
        let executor = Arc::new(SelectiveExecutor {
            fail: HashSet::new(),
        });
        let mut scheduler = WorkStealingScheduler::new(2, graph, executor);

        let summary = scheduler.run().unwrap();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.executed, 3);
        assert_eq!(summary.failed, 0);
    }

    #[test]
    fn work_stealing_propagates_failures() {
        let graph = chain_graph(&["a", "b", "c"]);
        let executor = Arc::new(SelectiveExecutor {
            fail: HashSet::from(["b".to_string()]),
        });
        let mut scheduler = WorkStealingScheduler::new(2, graph, executor);

        let summary = scheduler.run().unwrap();
        assert_eq!(summary.executed, 1, "only `a` completes");
        assert_eq!(summary.failed, 1, "`b` fails");
        assert_eq!(summary.cancelled, 1, "`c` is cancelled");
        assert_eq!(summary.failures.len(), 1);
    }

    #[test]
    fn test_work_stealing_diamond_fanout_stress() {
        let mut graph = BuildGraph::new();
        let root = graph.add_node(Task::new(
            "root".to_string(),
            "echo root".to_string(),
            CommandSpec::new("echo").arg("root"),
        ));

        let mut mid_nodes = Vec::new();
        for i in 0..64 {
            let mid = graph.add_node(Task::new(
                format!("mid_{i}"),
                format!("echo mid_{i}"),
                CommandSpec::new("echo").arg(format!("mid_{i}")),
            ));
            graph.add_dependency(root, mid).unwrap();
            mid_nodes.push(mid);
        }

        let sink = graph.add_node(Task::new(
            "sink".to_string(),
            "echo sink".to_string(),
            CommandSpec::new("echo").arg("sink"),
        ));
        for mid in mid_nodes {
            graph.add_dependency(mid, sink).unwrap();
        }

        let executor = Arc::new(SelectiveExecutor {
            fail: HashSet::new(),
        });
        let mut scheduler = WorkStealingScheduler::new(8, graph, executor);

        let summary = scheduler.run().unwrap();
        assert_eq!(summary.total, 66);
        assert_eq!(summary.executed, 66);
        assert_eq!(summary.failed, 0);
    }
}
