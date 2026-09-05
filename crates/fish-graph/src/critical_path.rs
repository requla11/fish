use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::GraphError;
use crate::graph::{BuildGraph, NodeId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CriticalPathReport {
    pub nodes: Vec<NodeId>,
    pub critical_path_duration_ms: u64,
    pub total_duration_ms: u64,
    pub speedup_ratio: f64,
}

pub struct CriticalPathAnalyzer;

impl CriticalPathAnalyzer {
    pub fn analyze<T>(
        graph: &BuildGraph<T>,
        node_durations_ms: &HashMap<NodeId, u64>,
    ) -> Result<CriticalPathReport, GraphError> {
        if graph.is_empty() {
            return Ok(CriticalPathReport {
                nodes: Vec::new(),
                critical_path_duration_ms: 0,
                total_duration_ms: 0,
                speedup_ratio: 1.0,
            });
        }

        let topo_order = graph.topological_order();
        let mut dist: HashMap<NodeId, u64> = HashMap::with_capacity(graph.len());
        let mut pred: HashMap<NodeId, Option<NodeId>> = HashMap::with_capacity(graph.len());

        for &node_id in &topo_order {
            let self_cost = *node_durations_ms.get(&node_id).unwrap_or(&0);
            let mut max_dep_dist = 0u64;
            let mut best_pred = None;

            for &dep_id in graph.deps(node_id)? {
                let dep_dist = *dist.get(&dep_id).unwrap_or(&0);
                if dep_dist >= max_dep_dist {
                    max_dep_dist = dep_dist;
                    best_pred = Some(dep_id);
                }
            }

            dist.insert(node_id, max_dep_dist + self_cost);
            pred.insert(node_id, best_pred);
        }

        let mut max_total_dist = 0u64;
        let mut sink_node = None;

        for &node_id in &topo_order {
            let node_dist = *dist.get(&node_id).unwrap_or(&0);
            if node_dist >= max_total_dist {
                max_total_dist = node_dist;
                sink_node = Some(node_id);
            }
        }

        let mut path = Vec::new();
        let mut current = sink_node;
        while let Some(curr_id) = current {
            path.push(curr_id);
            current = pred.get(&curr_id).copied().flatten();
        }
        path.reverse();

        let total_duration_ms: u64 = node_durations_ms.values().copied().sum();
        let speedup_ratio = if max_total_dist > 0 {
            total_duration_ms as f64 / max_total_dist as f64
        } else {
            1.0
        };

        Ok(CriticalPathReport {
            nodes: path,
            critical_path_duration_ms: max_total_dist,
            total_duration_ms,
            speedup_ratio,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let graph: BuildGraph<&str> = BuildGraph::new();
        let durations = HashMap::new();
        let report = CriticalPathAnalyzer::analyze(&graph, &durations).unwrap();
        assert!(report.nodes.is_empty());
        assert_eq!(report.critical_path_duration_ms, 0);
        assert_eq!(report.speedup_ratio, 1.0);
    }

    #[test]
    fn test_diamond_graph_critical_path() {
        let mut graph = BuildGraph::new();
        let a = graph.add_node("A");
        let b = graph.add_node("B");
        let c = graph.add_node("C");
        let d = graph.add_node("D");

        graph.add_dependency(a, b).unwrap();
        graph.add_dependency(a, c).unwrap();
        graph.add_dependency(b, d).unwrap();
        graph.add_dependency(c, d).unwrap();

        let mut durations = HashMap::new();
        durations.insert(a, 100);
        durations.insert(b, 50);
        durations.insert(c, 250);
        durations.insert(d, 50);

        let report = CriticalPathAnalyzer::analyze(&graph, &durations).unwrap();

        assert_eq!(report.nodes, vec![a, c, d]);
        assert_eq!(report.critical_path_duration_ms, 400);
        assert_eq!(report.total_duration_ms, 450);
        assert!((report.speedup_ratio - 1.125).abs() < 1e-6);
    }

    #[test]
    fn test_linear_graph_critical_path() {
        let mut graph = BuildGraph::new();
        let n1 = graph.add_node("compile");
        let n2 = graph.add_node("link");
        graph.add_dependency(n1, n2).unwrap();

        let mut durations = HashMap::new();
        durations.insert(n1, 300);
        durations.insert(n2, 200);

        let report = CriticalPathAnalyzer::analyze(&graph, &durations).unwrap();
        assert_eq!(report.nodes, vec![n1, n2]);
        assert_eq!(report.critical_path_duration_ms, 500);
        assert_eq!(report.total_duration_ms, 500);
        assert!((report.speedup_ratio - 1.0).abs() < 1e-6);
    }
}
