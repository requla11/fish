use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use fish_executor::{CommandSpec, ExecutorError, Task, TaskExecutor, TaskOutcome, TaskStatus};
use fish_graph::BuildGraph;
use fish_scheduler::{
    BuildSummary, ExecutionHeuristics, Scheduler, TaskTiming, WorkStealingScheduler,
};
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Default)]
struct NoopExecutor;

impl TaskExecutor for NoopExecutor {
    fn execute(&self, _task: &Task) -> Result<TaskOutcome, ExecutorError> {
        Ok(TaskOutcome {
            status: TaskStatus::Executed,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            duration: Duration::ZERO,
        })
    }
}

fn build_synthetic_dag(node_count: usize) -> BuildGraph<String> {
    let mut graph = BuildGraph::new();
    let mut node_ids = Vec::with_capacity(node_count);

    for i in 0..node_count {
        let id = graph.add_node(format!("task_{i}"));
        node_ids.push(id);
    }

    for i in 1..node_count {
        let parent = node_ids[i / 2];
        let child = node_ids[i];
        let _ = graph.add_dependency(parent, child);
    }

    graph
}

fn build_synthetic_task_graph(node_count: usize) -> BuildGraph<Task> {
    let mut graph = BuildGraph::new();
    let mut node_ids = Vec::with_capacity(node_count);

    for i in 0..node_count {
        let task = Task::new(
            format!("task_{i}"),
            format!("task_{i}"),
            CommandSpec::new("echo").arg("noop"),
        );
        let id = graph.add_node(task);
        node_ids.push(id);
    }

    for i in 1..node_count {
        let parent = node_ids[i / 2];
        let child = node_ids[i];
        let _ = graph.add_dependency(parent, child);
    }

    graph
}

fn bench_graph_topological_sort(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_topological_sort");

    for size in [50, 200, 1000] {
        let graph = build_synthetic_dag(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &graph, |b, g| {
            b.iter(|| {
                let order = g.topological_order();
                black_box(order);
            });
        });
    }

    group.finish();
}

fn bench_graph_ready_nodes(c: &mut Criterion) {
    let mut group = c.benchmark_group("dag_ready_nodes");

    for size in [50, 200, 1000] {
        let graph = build_synthetic_dag(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &graph, |b, g| {
            b.iter(|| {
                let ready = g.ready_nodes();
                black_box(ready);
            });
        });
    }

    group.finish();
}

fn bench_scheduler_dispatch_decision_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("scheduler_dispatch_overhead");
    let executor = NoopExecutor;

    for size in [50, 200, 1000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            b.iter(|| {
                let mut graph = build_synthetic_task_graph(n);
                let scheduler = Scheduler::new(4).with_critical_path_priority(true);
                let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
                black_box(summary);
            });
        });
    }

    group.finish();
}

fn bench_work_stealing_scheduler_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("work_stealing_scheduler");
    let executor = Arc::new(NoopExecutor);

    for size in [50, 200, 1000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &n| {
            b.iter(|| {
                let graph = build_synthetic_task_graph(n);
                let mut scheduler = WorkStealingScheduler::new(4, graph, executor.clone())
                    .with_heuristics(Arc::new(ExecutionHeuristics::default()));
                let summary = scheduler.run().unwrap();
                black_box(summary);
            });
        });
    }

    group.finish();
}

fn bench_critical_path_calculation(c: &mut Criterion) {
    let mut group = c.benchmark_group("critical_path_calculation");

    for size in [50, 200, 1000] {
        let graph = build_synthetic_task_graph(size);
        let timings: Vec<TaskTiming> = (0..size)
            .map(|i| {
                TaskTiming::new(
                    format!("task_{i}"),
                    Duration::from_micros((i % 10 + 1) as u64 * 50),
                    fish_graph::NodeId::from(i),
                )
            })
            .collect();

        let summary = BuildSummary {
            total: size,
            executed: size,
            cached: 0,
            failed: 0,
            cancelled: 0,
            duration: Duration::from_millis(100),
            workers: 4,
            failures: Vec::new(),
            timings,
        };

        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                let (total, path) = summary.critical_path(&graph);
                black_box((total, path));
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_graph_topological_sort,
    bench_graph_ready_nodes,
    bench_scheduler_dispatch_decision_overhead,
    bench_work_stealing_scheduler_throughput,
    bench_critical_path_calculation
);
criterion_main!(benches);
