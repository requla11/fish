use fish_executor::Task;
use fish_graph::BuildGraph;
use fish_scheduler::BuildSummary;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskRunReport {
    pub label: String,
    pub command: String,
    pub status: String,
    pub duration_ms: u64,
    pub cache_key: Option<String>,
    pub fingerprint: Option<String>,
    pub inputs_count: usize,
    pub artifacts_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupplyChainSummary {
    pub slsa_level: String,
    pub merkle_root_hash: String,
    pub ledger_records_count: usize,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnergyTelemetrySummary {
    pub energy_joules: f64,
    pub carbon_grams_co2: f64,
    pub avg_cpu_cores_utilized: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RunSummary {
    pub fish_version: String,
    pub run_id: String,
    pub timestamp: u64,
    pub duration_ms: u64,
    pub success: bool,
    pub workers: usize,
    pub total_tasks: usize,
    pub executed_tasks: usize,
    pub cached_tasks: usize,
    pub failed_tasks: usize,
    pub cancelled_tasks: usize,
    pub tasks: Vec<TaskRunReport>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supply_chain: Option<SupplyChainSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub energy_telemetry: Option<EnergyTelemetrySummary>,
}

impl RunSummary {
    pub fn with_supply_chain(mut self, supply_chain: SupplyChainSummary) -> Self {
        self.supply_chain = Some(supply_chain);
        self
    }

    pub fn with_energy_telemetry(mut self, energy: EnergyTelemetrySummary) -> Self {
        self.energy_telemetry = Some(energy);
        self
    }
    pub fn from_build(summary: &BuildSummary, graph: &BuildGraph<Task>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let mut hasher = blake3::Hasher::new();
        hasher.update(&timestamp.to_le_bytes());
        hasher.update(&(std::process::id() as u64).to_le_bytes());
        hasher.update(&(summary.total as u64).to_le_bytes());
        let run_id = hasher.finalize().to_hex()[..16].to_string();

        let mut timings_map = std::collections::HashMap::new();
        for t in &summary.timings {
            timings_map.insert(t.label.clone(), t.duration.as_millis() as u64);
        }

        let mut failure_map = std::collections::HashMap::new();
        for f in &summary.failures {
            failure_map.insert(f.label.clone(), f.stderr.clone());
        }

        let mut tasks = Vec::new();
        for node in graph.nodes() {
            let task = &node.payload;
            let status = match node.state {
                fish_graph::TaskState::Succeeded => "executed".to_string(),
                fish_graph::TaskState::Cached => "cached".to_string(),
                fish_graph::TaskState::Failed => "failed".to_string(),
                fish_graph::TaskState::Cancelled => "cancelled".to_string(),
                _ => "skipped".to_string(),
            };

            let duration_ms = timings_map.get(&task.label).copied().unwrap_or(0);
            let error = failure_map.get(&task.label).cloned();
            let cache_key = task.cache.as_ref().map(|c| c.key.clone());
            let fingerprint = task.cache.as_ref().map(|c| c.fingerprint.clone());

            tasks.push(TaskRunReport {
                label: task.label.clone(),
                command: task.description.clone(),
                status,
                duration_ms,
                cache_key,
                fingerprint,
                inputs_count: task.inputs.len(),
                artifacts_count: task.artifacts.len(),
                error,
            });
        }

        tasks.sort_by(|a, b| a.label.cmp(&b.label));

        Self {
            fish_version: env!("CARGO_PKG_VERSION").to_string(),
            run_id,
            timestamp,
            duration_ms: summary.duration.as_millis() as u64,
            success: summary.succeeded(),
            workers: summary.workers,
            total_tasks: summary.total,
            executed_tasks: summary.executed,
            cached_tasks: summary.cached,
            failed_tasks: summary.failed,
            cancelled_tasks: summary.cancelled,
            tasks,
            supply_chain: None,
            energy_telemetry: None,
        }
    }

    pub fn save_to_file(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json_bytes = serde_json::to_vec_pretty(self)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
        fs::write(path, json_bytes)
    }

    pub fn auto_save(
        &self,
        workspace_root: &Path,
        custom_file: Option<&Path>,
    ) -> io::Result<PathBuf> {
        if let Some(target) = custom_file {
            self.save_to_file(target)?;
            return Ok(target.to_path_buf());
        }

        let runs_dir = workspace_root.join(".fish").join("runs");
        fs::create_dir_all(&runs_dir)?;

        let latest_file = runs_dir.join("latest-summary.json");
        let id_file = runs_dir.join(format!("{}.json", self.run_id));

        self.save_to_file(&latest_file)?;
        let _ = self.save_to_file(&id_file);

        Ok(latest_file)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fish_executor::CommandSpec;

    #[test]
    fn test_run_summary_serialization() {
        let mut graph = BuildGraph::new();
        let spec = CommandSpec::new("cargo").arg("build");
        let task = Task::new("pkg-a", "cargo build", spec);
        let node_id = graph.add_node(task);
        let _ = graph.set_state(node_id, fish_graph::TaskState::Succeeded);

        let summary = BuildSummary {
            total: 1,
            executed: 1,
            cached: 0,
            failed: 0,
            cancelled: 0,
            duration: std::time::Duration::from_millis(150),
            workers: 4,
            failures: Vec::new(),
            timings: vec![fish_scheduler::TaskTiming::new(
                "pkg-a",
                std::time::Duration::from_millis(150),
                node_id,
            )],
        };

        let run_summary = RunSummary::from_build(&summary, &graph);
        assert_eq!(run_summary.total_tasks, 1);
        assert_eq!(run_summary.executed_tasks, 1);
        assert!(run_summary.success);
        assert_eq!(run_summary.tasks.len(), 1);
        assert_eq!(run_summary.tasks[0].status, "executed");
        assert_eq!(run_summary.tasks[0].duration_ms, 150);

        let temp = tempfile::tempdir().unwrap();
        let out_file = temp.path().join("summary.json");
        run_summary.save_to_file(&out_file).unwrap();
        assert!(out_file.exists());

        let read_back: RunSummary = serde_json::from_slice(&fs::read(&out_file).unwrap()).unwrap();
        assert_eq!(read_back.run_id, run_summary.run_id);
    }

    #[test]
    fn test_run_summary_with_supply_chain_and_energy_telemetry() {
        let graph = BuildGraph::<Task>::new();
        let summary = BuildSummary {
            total: 0,
            executed: 0,
            cached: 0,
            failed: 0,
            cancelled: 0,
            duration: std::time::Duration::from_millis(50),
            workers: 2,
            failures: Vec::new(),
            timings: Vec::new(),
        };

        let run_summary = RunSummary::from_build(&summary, &graph)
            .with_supply_chain(SupplyChainSummary {
                slsa_level: "SLSA_BUILD_LEVEL_3".to_string(),
                merkle_root_hash: "blake3:abc123merkle".to_string(),
                ledger_records_count: 5,
                signature: Some("ed25519:test_signature".to_string()),
            })
            .with_energy_telemetry(EnergyTelemetrySummary {
                energy_joules: 1250.5,
                carbon_grams_co2: 0.08,
                avg_cpu_cores_utilized: 3.5,
            });

        let serialized = serde_json::to_string_pretty(&run_summary).unwrap();
        assert!(serialized.contains("SLSA_BUILD_LEVEL_3"));
        assert!(serialized.contains("blake3:abc123merkle"));
        assert!(serialized.contains("1250.5"));
        assert!(serialized.contains("carbon_grams_co2"));
    }
}
