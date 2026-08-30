use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use fish_executor::{CommandSpec, ExecutorError, Task, TaskExecutor, TaskOutcome, TaskStatus};
use fish_graph::{BuildGraph, NodeId, TaskState};
use fish_scheduler::{ExecutionHeuristics, Scheduler, WorkStealingScheduler};
use std::hint::black_box;
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone, Default)]
struct SimulatedTaskExecutor;

impl TaskExecutor for SimulatedTaskExecutor {
    fn execute(&self, task: &Task) -> Result<TaskOutcome, ExecutorError> {
        let simulated_us = if task.label.starts_with("codegen") {
            20
        } else if task.label.starts_with("compile") {
            80
        } else if task.label.starts_with("link") {
            40
        } else {
            10
        };

        if simulated_us > 0 {
            std::thread::sleep(Duration::from_micros(simulated_us));
        }

        Ok(TaskOutcome {
            status: TaskStatus::Executed,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            duration: Duration::from_micros(simulated_us),
        })
    }
}

fn build_polyglot_monorepo_graph(scale: usize) -> BuildGraph<Task> {
    let mut graph = BuildGraph::new();

    let codegen_id = graph.add_node(Task::new(
        "codegen_schema",
        "codegen_schema",
        CommandSpec::new("generator").arg("proto"),
    ));

    let mut compile_nodes = Vec::with_capacity(scale * 4);
    for i in 0..scale {
        let rust_node = graph.add_node(Task::new(
            format!("compile_rust_{i}"),
            format!("compile_rust_{i}"),
            CommandSpec::new("rustc"),
        ));
        let cc_node = graph.add_node(Task::new(
            format!("compile_cc_{i}"),
            format!("compile_cc_{i}"),
            CommandSpec::new("clang"),
        ));
        let ts_node = graph.add_node(Task::new(
            format!("compile_ts_{i}"),
            format!("compile_ts_{i}"),
            CommandSpec::new("tsc"),
        ));
        let go_node = graph.add_node(Task::new(
            format!("compile_go_{i}"),
            format!("compile_go_{i}"),
            CommandSpec::new("go"),
        ));

        let _ = graph.add_dependency(codegen_id, rust_node);
        let _ = graph.add_dependency(codegen_id, cc_node);
        let _ = graph.add_dependency(codegen_id, ts_node);
        let _ = graph.add_dependency(codegen_id, go_node);

        compile_nodes.push(rust_node);
        compile_nodes.push(cc_node);
        compile_nodes.push(ts_node);
        compile_nodes.push(go_node);
    }

    let mut link_nodes = Vec::with_capacity(scale);
    for i in 0..scale {
        let link_node = graph.add_node(Task::new(
            format!("link_app_{i}"),
            format!("link_app_{i}"),
            CommandSpec::new("linker"),
        ));
        let r_id = compile_nodes[i * 4];
        let c_id = compile_nodes[i * 4 + 1];
        let t_id = compile_nodes[i * 4 + 2];
        let g_id = compile_nodes[i * 4 + 3];

        let _ = graph.add_dependency(r_id, link_node);
        let _ = graph.add_dependency(c_id, link_node);
        let _ = graph.add_dependency(t_id, link_node);
        let _ = graph.add_dependency(g_id, link_node);

        link_nodes.push(link_node);
    }

    let test_all_id = graph.add_node(Task::new(
        "integration_test_all",
        "integration_test_all",
        CommandSpec::new("tester"),
    ));
    for link_id in link_nodes {
        let _ = graph.add_dependency(link_id, test_all_id);
    }

    graph
}

fn execute_simulated_ninja_wavefront(
    graph: &mut BuildGraph<Task>,
    executor: &SimulatedTaskExecutor,
    workers: usize,
) {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .unwrap();

    while graph
        .nodes()
        .iter()
        .any(|n| n.state == TaskState::Pending || n.state == TaskState::Running)
    {
        let ready = graph.ready_nodes();
        if ready.is_empty() {
            break;
        }

        let tasks: Vec<(NodeId, Task)> = ready
            .iter()
            .map(|&id| (id, graph.node(id).unwrap().payload.clone()))
            .collect();

        for &id in &ready {
            let _ = graph.set_state(id, TaskState::Running);
        }

        pool.scope(|s| {
            for (_, task) in &tasks {
                s.spawn(|_| {
                    let _ = executor.execute(task);
                });
            }
        });

        for &(id, _) in &tasks {
            let _ = graph.set_state(id, TaskState::Succeeded);
        }
    }
}

fn execute_simulated_bazel_barrier_phased(
    graph: &mut BuildGraph<Task>,
    executor: &SimulatedTaskExecutor,
    workers: usize,
) {
    let topo = graph.topological_order();
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(workers)
        .build()
        .unwrap();

    let mut chunks = Vec::new();
    let mut current_chunk = Vec::new();
    let chunk_size = (topo.len() / 4).max(1);

    for id in topo {
        current_chunk.push(id);
        if current_chunk.len() >= chunk_size {
            chunks.push(std::mem::take(&mut current_chunk));
        }
    }
    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    for chunk in chunks {
        let tasks: Vec<(NodeId, Task)> = chunk
            .iter()
            .map(|&id| (id, graph.node(id).unwrap().payload.clone()))
            .collect();

        for &id in &chunk {
            let _ = graph.set_state(id, TaskState::Running);
        }

        pool.scope(|s| {
            for (_, task) in &tasks {
                s.spawn(|_| {
                    let _ = executor.execute(task);
                });
            }
        });

        for &(id, _) in &tasks {
            let _ = graph.set_state(id, TaskState::Succeeded);
        }
    }
}

fn bench_peer_comparison_matrix(c: &mut Criterion) {
    let mut group = c.benchmark_group("peer_build_scheduler_comparison");
    let executor = SimulatedTaskExecutor;
    let arc_executor = Arc::new(SimulatedTaskExecutor);

    for scale in [5, 25, 100] {
        let total_tasks = 1 + scale * 4 + scale + 1;
        group.throughput(Throughput::Elements(total_tasks as u64));

        group.bench_with_input(
            BenchmarkId::new("fish_work_stealing", total_tasks),
            &scale,
            |b, &s| {
                b.iter(|| {
                    let graph = build_polyglot_monorepo_graph(s);
                    let mut scheduler = WorkStealingScheduler::new(8, graph, arc_executor.clone())
                        .with_heuristics(Arc::new(ExecutionHeuristics::default()));
                    let summary = scheduler.run().unwrap();
                    black_box(summary);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("fish_critical_path", total_tasks),
            &scale,
            |b, &s| {
                b.iter(|| {
                    let mut graph = build_polyglot_monorepo_graph(s);
                    let scheduler = Scheduler::new(8).with_critical_path_priority(true);
                    let summary = scheduler.run(&mut graph, &executor, |_, _| {}).unwrap();
                    black_box(summary);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("simulated_ninja_wavefront", total_tasks),
            &scale,
            |b, &s| {
                b.iter(|| {
                    let mut graph = build_polyglot_monorepo_graph(s);
                    execute_simulated_ninja_wavefront(&mut graph, &executor, 8);
                    black_box(&graph);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("simulated_bazel_barrier_phased", total_tasks),
            &scale,
            |b, &s| {
                b.iter(|| {
                    let mut graph = build_polyglot_monorepo_graph(s);
                    execute_simulated_bazel_barrier_phased(&mut graph, &executor, 8);
                    black_box(&graph);
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_peer_comparison_matrix);
criterion_main!(benches);
