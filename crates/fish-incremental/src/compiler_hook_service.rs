use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::ast_cache::AstCacheIndex;
use crate::compiler_hooks::{
    DiffResult, ModuleSnapshot, RebuildDecision, compute_rebuild_decision, diff_snapshots,
    parse_module, snapshot_to_ast_subtrees,
};
use crate::semantic_impact::SemanticImpactGraph;
use crate::ts_hooks::{TsDialect, parse_ts_module};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRebuildPlan {
    pub files_analyzed: usize,
    pub files_unchanged: usize,
    pub files_with_body_only_changes: usize,
    pub files_with_signature_changes: usize,
    pub total_items: usize,
    pub items_to_rebuild: usize,
    pub items_safe_to_skip: usize,
    pub cascade_count: usize,
    pub affected_tests: Vec<String>,
    pub overall_reuse_ratio: f64,
    pub per_file: Vec<FileRebuildPlan>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRebuildPlan {
    pub file_path: PathBuf,
    pub diff: DiffResult,
    pub decision: RebuildDecision,
}

pub struct CompilerHookService {
    snapshot_cache: HashMap<PathBuf, ModuleSnapshot>,
    ast_cache: AstCacheIndex,
    impact_graph: SemanticImpactGraph,
}

impl CompilerHookService {
    pub fn new() -> Self {
        Self {
            snapshot_cache: HashMap::new(),
            ast_cache: AstCacheIndex::new(),
            impact_graph: SemanticImpactGraph::new(),
        }
    }

    pub fn with_impact_graph(mut self, graph: SemanticImpactGraph) -> Self {
        self.impact_graph = graph;
        self
    }

    pub fn register_symbol_test_mapping(&mut self, symbol: &str, test_target: &str) {
        self.impact_graph.map_symbol_to_test(symbol, test_target);
    }

    fn normalize_path(path: &Path) -> PathBuf {
        PathBuf::from(path.to_string_lossy().replace('\\', "/"))
    }

    pub fn analyze_file(&mut self, file_path: &Path) -> Result<Option<FileRebuildPlan>, String> {
        let norm_path = Self::normalize_path(file_path);

        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if let Some(old_snapshot) = self.snapshot_cache.remove(&norm_path) {
                    let mut diffs = Vec::new();
                    let mut must_rebuild = Vec::new();
                    for item in &old_snapshot.items {
                        diffs.push(crate::compiler_hooks::ItemDiff {
                            name: item.name.clone(),
                            kind: item.kind.clone(),
                            change: crate::compiler_hooks::ChangeKind::Removed,
                        });
                        must_rebuild.push(item.name.clone());
                    }
                    must_rebuild.sort();

                    let mut affected_tests = if diffs.is_empty() {
                        Vec::new()
                    } else {
                        self.impact_graph.find_impacted_tests(&must_rebuild)
                    };
                    affected_tests.sort();

                    return Ok(Some(FileRebuildPlan {
                        file_path: norm_path.clone(),
                        diff: DiffResult {
                            file_path: norm_path,
                            diffs,
                            module_hash_changed: true,
                        },
                        decision: RebuildDecision {
                            must_rebuild,
                            safe_to_skip: Vec::new(),
                            cascade_targets: Vec::new(),
                            affected_tests,
                            reuse_ratio: 0.0,
                        },
                    }));
                }
                return Ok(None);
            }
            Err(e) => return Err(format!("failed to read {}: {e}", file_path.display())),
        };

        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");

        let new_snapshot = if TsDialect::from_extension(ext).is_some() {
            parse_ts_module(&norm_path, &content)?
        } else if ext == "rs" {
            parse_module(&norm_path, &content)?
        } else {
            return Err(format!("Unsupported file extension: {}", ext));
        };

        let subtrees = snapshot_to_ast_subtrees(&new_snapshot);
        let file_key = norm_path.display().to_string();
        self.ast_cache.record_file_subtrees(&file_key, subtrees);

        let plan = if let Some(old_snapshot) = self.snapshot_cache.get(&norm_path) {
            let diff = diff_snapshots(old_snapshot, &new_snapshot);
            if diff.is_clean() {
                None
            } else {
                let decision =
                    compute_rebuild_decision(&diff, &new_snapshot, Some(&self.impact_graph));
                Some(FileRebuildPlan {
                    file_path: norm_path.clone(),
                    diff,
                    decision,
                })
            }
        } else {
            let diff = DiffResult {
                file_path: norm_path.clone(),
                diffs: new_snapshot
                    .items
                    .iter()
                    .map(|item| crate::compiler_hooks::ItemDiff {
                        name: item.name.clone(),
                        kind: item.kind.clone(),
                        change: crate::compiler_hooks::ChangeKind::Added,
                    })
                    .collect(),
                module_hash_changed: true,
            };
            let decision = compute_rebuild_decision(&diff, &new_snapshot, Some(&self.impact_graph));
            Some(FileRebuildPlan {
                file_path: norm_path.clone(),
                diff,
                decision,
            })
        };

        self.snapshot_cache.insert(norm_path, new_snapshot);
        Ok(plan)
    }

    pub fn analyze_workspace(
        &mut self,
        changed_files: &[PathBuf],
    ) -> Result<WorkspaceRebuildPlan, String> {
        let mut files_analyzed = 0usize;
        let mut files_unchanged = 0usize;
        let mut files_body_only = 0usize;
        let mut files_sig = 0usize;

        let mut diffs_by_file = HashMap::new();

        for file_path in changed_files {
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "rs" && TsDialect::from_extension(ext).is_none() {
                continue;
            }
            files_analyzed += 1;

            match self.analyze_file(file_path)? {
                None => {
                    files_unchanged += 1;
                }
                Some(plan) => {
                    if plan.diff.has_signature_changes() || plan.diff.module_hash_changed {
                        files_sig += 1;
                    } else {
                        files_body_only += 1;
                    }
                    diffs_by_file.insert(plan.file_path.clone(), plan.diff.clone());
                }
            }
        }

        let mut global_edges: HashMap<String, std::collections::BTreeSet<String>> = HashMap::new();
        let mut all_names = std::collections::HashSet::new();
        for snap in self.snapshot_cache.values() {
            for item in &snap.items {
                all_names.insert(item.name.clone());
            }
            for (from, to) in &snap.edges {
                global_edges
                    .entry(from.clone())
                    .or_default()
                    .extend(to.iter().cloned());
            }
        }

        let mut must_rebuild_global = std::collections::HashSet::new();
        let mut cascade_targets_global = std::collections::HashSet::new();
        let mut diff_items_with_module_changes = Vec::new();

        for (path, diff) in &diffs_by_file {
            let mut file_items = Vec::new();
            if let Some(snap) = self.snapshot_cache.get(path) {
                file_items = snap
                    .items
                    .iter()
                    .map(|i| (i.name.clone(), i.kind.clone()))
                    .collect();
            }

            if diff.module_hash_changed {
                for (name, kind) in file_items {
                    must_rebuild_global.insert(name.clone());
                    diff_items_with_module_changes.push(crate::compiler_hooks::ItemDiff {
                        name,
                        kind,
                        change: crate::compiler_hooks::ChangeKind::SignatureModified, // Treat module-level changes as signature changes to force cascade
                    });
                }
            }

            for diff_item in &diff.diffs {
                must_rebuild_global.insert(diff_item.name.clone());
                diff_items_with_module_changes.push(diff_item.clone());
            }
        }

        for diff_item in diff_items_with_module_changes {
            if diff_item.change == crate::compiler_hooks::ChangeKind::SignatureModified
                || diff_item.change == crate::compiler_hooks::ChangeKind::Removed
            {
                let deps =
                    crate::compiler_hooks::transitive_dependents(&diff_item.name, &global_edges);
                for dep in deps {
                    must_rebuild_global.insert(dep.clone());
                    cascade_targets_global.insert(dep);
                }
            }
        }

        let items_rebuild = must_rebuild_global.len();
        let total_items = all_names.len();
        let items_skip = total_items.saturating_sub(items_rebuild);

        let mut must_rebuild_vec: Vec<String> = must_rebuild_global.into_iter().collect();
        must_rebuild_vec.sort();

        let mut all_tests = self.impact_graph.find_impacted_tests(&must_rebuild_vec);
        all_tests.sort();

        let cascade_count = cascade_targets_global.len();
        let total_for_ratio = items_rebuild + items_skip;
        let overall_reuse_ratio = if total_for_ratio == 0 {
            1.0
        } else {
            items_skip as f64 / total_for_ratio as f64
        };

        // For simplicity, we just package the modified diffs back into FileRebuildPlan.
        // We could also distribute the global rebuild back to files, but the WorkspaceRebuildPlan
        // metrics are now global and correct.
        let mut per_file = Vec::new();
        for (path, diff) in diffs_by_file {
            per_file.push(FileRebuildPlan {
                file_path: path,
                diff,
                decision: RebuildDecision {
                    must_rebuild: must_rebuild_vec.clone(),
                    safe_to_skip: Vec::new(), // omitted at file level since it's global
                    cascade_targets: cascade_targets_global.iter().cloned().collect(),
                    affected_tests: all_tests.clone(),
                    reuse_ratio: overall_reuse_ratio,
                },
            });
        }

        Ok(WorkspaceRebuildPlan {
            files_analyzed,
            files_unchanged,
            files_with_body_only_changes: files_body_only,
            files_with_signature_changes: files_sig,
            total_items,
            items_to_rebuild: items_rebuild,
            items_safe_to_skip: items_skip,
            cascade_count,
            affected_tests: all_tests,
            overall_reuse_ratio,
            per_file,
        })
    }

    pub fn snapshot_cache(&self) -> &HashMap<PathBuf, ModuleSnapshot> {
        &self.snapshot_cache
    }

    pub fn ast_cache(&self) -> &AstCacheIndex {
        &self.ast_cache
    }
}

impl Default for CompilerHookService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_rs(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::with_suffix(".rs").unwrap();
        write!(f, "{content}").unwrap();
        f
    }

    #[test]
    fn test_service_first_analysis_marks_all_as_added() {
        let f = write_rs("fn hello() {}\nstruct World;");
        let mut svc = CompilerHookService::new();

        let plan = svc.analyze_file(f.path()).unwrap().unwrap();
        assert_eq!(plan.diff.diffs.len(), 2);
        assert!(
            plan.diff
                .diffs
                .iter()
                .all(|d| d.change == crate::compiler_hooks::ChangeKind::Added)
        );
    }

    #[test]
    fn test_service_second_analysis_detects_no_change() {
        let f = write_rs("fn hello() {}");
        let mut svc = CompilerHookService::new();

        let _ = svc.analyze_file(f.path()).unwrap();
        let plan = svc.analyze_file(f.path()).unwrap();
        assert!(plan.is_none());
    }

    #[test]
    fn test_service_detects_body_modification() {
        let f = write_rs("fn greet() { println!(\"A\"); }");
        let mut svc = CompilerHookService::new();
        let _ = svc.analyze_file(f.path()).unwrap();

        std::fs::write(f.path(), "fn greet() { println!(\"B\"); }").unwrap();

        let plan = svc.analyze_file(f.path()).unwrap().unwrap();
        assert_eq!(plan.diff.diffs.len(), 1);
        assert_eq!(
            plan.diff.diffs[0].change,
            crate::compiler_hooks::ChangeKind::BodyModified
        );
    }

    #[test]
    fn test_workspace_analysis() {
        let f1 = write_rs("fn alpha() -> i32 { 1 }");
        let f2 = write_rs("fn beta() -> i32 { 2 }");

        let mut svc = CompilerHookService::new();
        let _ = svc
            .analyze_workspace(&[f1.path().to_path_buf(), f2.path().to_path_buf()])
            .unwrap();

        std::fs::write(f1.path(), "fn alpha() -> i32 { 999 }").unwrap();

        let plan = svc
            .analyze_workspace(&[f1.path().to_path_buf(), f2.path().to_path_buf()])
            .unwrap();
        assert_eq!(plan.files_with_body_only_changes, 1);
        assert_eq!(plan.files_unchanged, 1);
        assert!(plan.overall_reuse_ratio >= 0.0);
    }

    #[test]
    fn test_workspace_with_test_impact() {
        let f = write_rs("fn tax(x: f64) -> f64 { x * 0.1 }");
        let mut svc = CompilerHookService::new();
        svc.register_symbol_test_mapping("tax", "tests::tax_rate");

        let _ = svc.analyze_file(f.path()).unwrap();

        std::fs::write(f.path(), "fn tax(x: f64) -> f64 { x * 0.2 }").unwrap();

        let plan = svc.analyze_file(f.path()).unwrap().unwrap();
        assert!(
            plan.decision
                .affected_tests
                .contains(&"tests::tax_rate".to_string())
        );
    }

    #[test]
    fn test_ast_cache_integration() {
        let f = write_rs("fn cached_fn() -> bool { true }");
        let mut svc = CompilerHookService::new();
        let _ = svc.analyze_file(f.path()).unwrap();

        let key = f.path().display().to_string();
        let subtrees = svc.ast_cache().file_trees.get(&key).unwrap();
        assert!(subtrees.iter().any(|s| s.symbol_name == "cached_fn"));
    }
}
