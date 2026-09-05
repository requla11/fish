use fish_cache::LocalCache;
use fish_executor::Task;
use fish_graph::BuildGraph;
use serde::{Deserialize, Serialize};
use std::process::ExitCode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DryRunTaskReport {
    pub label: String,
    pub command: String,
    pub dependencies: Vec<String>,
    pub cache_status: String,
    pub cache_key: Option<String>,
    pub fingerprint: Option<String>,
    pub inputs_count: usize,
    pub artifacts_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DryRunReport {
    pub total_tasks: usize,
    pub cached_tasks: usize,
    pub tasks_to_execute: usize,
    pub execution_order: Vec<DryRunTaskReport>,
}

pub fn compute_dry_run_report(
    task_graph: &BuildGraph<Task>,
    cache: Option<&LocalCache>,
) -> DryRunReport {
    let topo_order = task_graph.topological_order();
    let mut execution_order = Vec::with_capacity(topo_order.len());
    let mut cached_tasks = 0;
    let mut tasks_to_execute = 0;

    for &node_id in &topo_order {
        let Some(node) = task_graph.node(node_id) else {
            continue;
        };
        let task = &node.payload;

        let dependencies = task_graph
            .deps(node_id)
            .map(|deps| {
                deps.iter()
                    .filter_map(|&dep_id| task_graph.node(dep_id).map(|n| n.payload.label.clone()))
                    .collect()
            })
            .unwrap_or_default();

        let (cache_status, cache_key, fingerprint) = if let Some(ref c) = task.cache {
            let is_hit = cache
                .map(|cache_engine| cache_engine.matches(&c.key, &c.fingerprint))
                .unwrap_or(false);
            let status = if is_hit { "HIT" } else { "MISS" };
            (
                status.to_string(),
                Some(c.key.clone()),
                Some(c.fingerprint.clone()),
            )
        } else {
            ("MISS".to_string(), None, None)
        };

        if cache_status == "HIT" {
            cached_tasks += 1;
        } else {
            tasks_to_execute += 1;
        }

        execution_order.push(DryRunTaskReport {
            label: task.label.clone(),
            command: task.spec.command_line(),
            dependencies,
            cache_status,
            cache_key,
            fingerprint,
            inputs_count: task.inputs.len(),
            artifacts_count: task.artifacts.len(),
        });
    }

    DryRunReport {
        total_tasks: execution_order.len(),
        cached_tasks,
        tasks_to_execute,
        execution_order,
    }
}

pub fn execute_dry_run(
    task_graph: &BuildGraph<Task>,
    cache: Option<&LocalCache>,
    mode: &str,
) -> ExitCode {
    let report = compute_dry_run_report(task_graph, cache);

    if mode.eq_ignore_ascii_case("json") {
        match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(err) => {
                eprintln!("error: failed to serialize dry run report: {err}");
                return ExitCode::FAILURE;
            }
        }
    } else {
        println!("🐟 Fish Dry Run Execution Plan");
        println!("============================================================");
        println!("Tasks in topological execution order:");
        for (idx, task) in report.execution_order.iter().enumerate() {
            println!("  {}. [{}] {}", idx + 1, task.cache_status, task.label);
            println!("     Command: {}", task.command);
            if !task.dependencies.is_empty() {
                println!("     Dependencies: [{}]", task.dependencies.join(", "));
            }
            if let Some(ref key) = task.cache_key {
                println!("     Cache Key: {key}");
            }
            println!(
                "     Inputs: {} | Artifacts: {}",
                task.inputs_count, task.artifacts_count
            );
        }
        println!("============================================================");
        println!("Dry Run Summary:");
        println!("  Total tasks:       {}", report.total_tasks);
        println!("  Cached (Hit):      {}", report.cached_tasks);
        println!("  To execute (Miss): {}", report.tasks_to_execute);
    }

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;
    use fish_executor::{CacheEntry, CommandSpec};

    #[test]
    fn test_dry_run_computation_with_cache() {
        let mut graph = BuildGraph::new();

        let t1 = Task::new(
            "compile",
            "compile app",
            CommandSpec::new("rustc").arg("main.rs"),
        )
        .with_cache(CacheEntry {
            key: "cache-k1".to_string(),
            fingerprint: "fp-1234".to_string(),
        });
        let id1 = graph.add_node(t1);

        let t2 = Task::new("link", "link app", CommandSpec::new("ld").arg("main.o")).with_cache(
            CacheEntry {
                key: "cache-k2".to_string(),
                fingerprint: "fp-5678".to_string(),
            },
        );
        let id2 = graph.add_node(t2);

        graph.add_dependency(id1, id2).unwrap();

        let tmp = tempfile::tempdir().unwrap();
        let cache = LocalCache::new(tmp.path()).unwrap();
        cache.put("cache-k1", "fp-1234").unwrap();

        let report = compute_dry_run_report(&graph, Some(&cache));
        assert_eq!(report.total_tasks, 2);
        assert_eq!(report.cached_tasks, 1);
        assert_eq!(report.tasks_to_execute, 1);

        assert_eq!(report.execution_order[0].label, "compile");
        assert_eq!(report.execution_order[0].cache_status, "HIT");
        assert_eq!(report.execution_order[0].dependencies, Vec::<String>::new());

        assert_eq!(report.execution_order[1].label, "link");
        assert_eq!(report.execution_order[1].cache_status, "MISS");
        assert_eq!(report.execution_order[1].dependencies, vec!["compile"]);
    }

    #[test]
    fn test_dry_run_json_serialization() {
        let mut graph = BuildGraph::new();
        let t1 = Task::new("test", "run tests", CommandSpec::new("cargo").arg("test"));
        graph.add_node(t1);

        let report = compute_dry_run_report(&graph, None);
        let serialized = serde_json::to_string(&report).unwrap();
        let deserialized: DryRunReport = serde_json::from_str(&serialized).unwrap();
        assert_eq!(report, deserialized);
    }
}
