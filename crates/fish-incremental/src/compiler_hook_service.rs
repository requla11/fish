use std::collections::HashMap;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

use crate::ast_cache::AstCacheIndex;
use crate::compiler_hooks::{
    ModuleSnapshot, DiffResult, RebuildDecision,
    parse_module, diff_snapshots, compute_rebuild_decision,
    snapshot_to_ast_subtrees,
};
use crate::ts_hooks::{TsDialect, parse_ts_module};
use crate::semantic_impact::SemanticImpactGraph;

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

    pub fn analyze_file(&mut self, file_path: &Path) -> Result<Option<FileRebuildPlan>, String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("failed to read {}: {e}", file_path.display()))?;

        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        
        let new_snapshot = if TsDialect::from_extension(ext).is_some() {
            parse_ts_module(file_path, &content)?
        } else if ext == "rs" {
            parse_module(file_path, &content)?
        } else {
            return Err(format!("Unsupported file extension: {}", ext));
        };

        let subtrees = snapshot_to_ast_subtrees(&new_snapshot);
        let file_key = file_path.display().to_string();
        self.ast_cache.record_file_subtrees(&file_key, subtrees);

        let plan = if let Some(old_snapshot) = self.snapshot_cache.get(file_path) {
            let diff = diff_snapshots(old_snapshot, &new_snapshot);
            if diff.is_clean() {
                None
            } else {
                let decision = compute_rebuild_decision(
                    &diff,
                    &new_snapshot,
                    Some(&self.impact_graph),
                );
                Some(FileRebuildPlan {
                    file_path: file_path.to_path_buf(),
                    diff,
                    decision,
                })
            }
        } else {
            let diff = DiffResult {
                file_path: file_path.to_path_buf(),
                diffs: new_snapshot.items.iter().map(|item| {
                    crate::compiler_hooks::ItemDiff {
                        name: item.name.clone(),
                        kind: item.kind.clone(),
                        change: crate::compiler_hooks::ChangeKind::Added,
                    }
                }).collect(),
                module_hash_changed: true,
            };
            let decision = compute_rebuild_decision(
                &diff,
                &new_snapshot,
                Some(&self.impact_graph),
            );
            Some(FileRebuildPlan {
                file_path: file_path.to_path_buf(),
                diff,
                decision,
            })
        };

        self.snapshot_cache.insert(file_path.to_path_buf(), new_snapshot);
        Ok(plan)
    }

    pub fn analyze_workspace(&mut self, changed_files: &[PathBuf]) -> Result<WorkspaceRebuildPlan, String> {
        let mut per_file = Vec::new();
        let mut files_unchanged = 0usize;
        let mut files_body_only = 0usize;
        let mut files_sig = 0usize;
        let mut total_items = 0usize;
        let mut items_rebuild = 0usize;
        let mut items_skip = 0usize;
        let mut cascade_count = 0usize;
        let mut all_tests: Vec<String> = Vec::new();

        for file_path in changed_files {
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "rs" && TsDialect::from_extension(ext).is_none() {
                continue;
            }

            match self.analyze_file(file_path)? {
                None => {
                    files_unchanged += 1;
                }
                Some(plan) => {
                    total_items += plan.decision.must_rebuild.len() + plan.decision.safe_to_skip.len();
                    items_rebuild += plan.decision.must_rebuild.len();
                    items_skip += plan.decision.safe_to_skip.len();
                    cascade_count += plan.decision.cascade_targets.len();

                    for t in &plan.decision.affected_tests {
                        if !all_tests.contains(t) {
                            all_tests.push(t.clone());
                        }
                    }

                    if plan.diff.has_signature_changes() {
                        files_sig += 1;
                    } else {
                        files_body_only += 1;
                    }

                    per_file.push(plan);
                }
            }
        }

        let total_for_ratio = items_rebuild + items_skip;
        let overall_reuse_ratio = if total_for_ratio == 0 {
            1.0
        } else {
            items_skip as f64 / total_for_ratio as f64
        };

        Ok(WorkspaceRebuildPlan {
            files_analyzed: changed_files.len(),
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
        assert!(plan.diff.diffs.iter().all(|d| d.change == crate::compiler_hooks::ChangeKind::Added));
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
        assert_eq!(plan.diff.diffs[0].change, crate::compiler_hooks::ChangeKind::BodyModified);
    }

    #[test]
    fn test_workspace_analysis() {
        let f1 = write_rs("fn alpha() -> i32 { 1 }");
        let f2 = write_rs("fn beta() -> i32 { 2 }");

        let mut svc = CompilerHookService::new();
        let _ = svc.analyze_workspace(&[f1.path().to_path_buf(), f2.path().to_path_buf()]).unwrap();

        std::fs::write(f1.path(), "fn alpha() -> i32 { 999 }").unwrap();

        let plan = svc.analyze_workspace(&[f1.path().to_path_buf(), f2.path().to_path_buf()]).unwrap();
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
        assert!(plan.decision.affected_tests.contains(&"tests::tax_rate".to_string()));
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
