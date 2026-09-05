use std::collections::BTreeSet;

/// Selects the minimal set of test targets that can be affected by a change.
///
/// Uses the semantic impact graph when symbol mappings are available and
/// falls back to path-prefix heuristics (crate-local tests for changed
/// crates) otherwise.
#[derive(Debug, Default)]
pub struct TestSelector {
    /// symbol -> impacted test targets
    graph: std::collections::HashMap<String, BTreeSet<String>>,
    /// crate directory name -> default test target
    crate_tests: std::collections::HashMap<String, Vec<String>>,
}

impl TestSelector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `symbol` is exercised by `test_target`.
    pub fn map_symbol_to_test(&mut self, symbol: &str, test_target: &str) {
        self.graph
            .entry(symbol.to_string())
            .or_default()
            .insert(test_target.to_string());
    }

    /// Register the default test targets owned by a crate directory.
    pub fn register_crate(&mut self, crate_dir: &str, targets: &[&str]) {
        self.crate_tests
            .entry(crate_dir.to_string())
            .or_default()
            .extend(targets.iter().map(|s| s.to_string()));
    }

    /// Choose tests to run given modified symbols and changed file paths.
    ///
    /// Returns a deduplicated, deterministically ordered list. When both
    /// inputs are empty nothing should run.
    pub fn select(&self, modified_symbols: &[String], changed_paths: &[String]) -> Vec<String> {
        let mut selected: BTreeSet<String> = BTreeSet::new();

        for symbol in modified_symbols {
            if let Some(tests) = self.graph.get(symbol) {
                selected.extend(tests.iter().cloned());
            }
        }

        for path in changed_paths {
            let normalized = path.replace('\\', "/");
            for (dir, targets) in &self.crate_tests {
                if normalized.contains(&format!("/crates/{dir}/"))
                    || normalized.starts_with(&format!("crates/{dir}/"))
                {
                    selected.extend(targets.iter().cloned());
                }
            }
            // Integration tests directories map to their own file target.
            if let Some(name) = integration_test_name(&normalized) {
                selected.insert(format!("test:{name}"));
            }
        }

        selected.into_iter().collect()
    }

    /// Ratio of skipped tests — reported so callers can log savings.
    pub fn skip_ratio(&self, selected: usize, total: usize) -> f64 {
        if total == 0 {
            return 0.0;
        }
        1.0 - (selected.min(total) as f64 / total as f64)
    }
}

/// Extract the stem of an integration test under `tests/`.
fn integration_test_name(path: &str) -> Option<&str> {
    let rest = if let Some(stripped) = path.strip_prefix("tests/") {
        stripped
    } else {
        let idx = path.find("/tests/")?;
        &path[idx + "/tests/".len()..]
    };
    let stem = rest.split('/').next()?;
    if stem.ends_with(".rs") && stem != "mod.rs" {
        Some(stem.trim_end_matches(".rs"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_inputs_run_nothing() {
        let sel = TestSelector::new();
        assert!(sel.select(&[], &[]).is_empty());
    }

    #[test]
    fn symbol_mapping_selects_impacted_test() {
        let mut sel = TestSelector::new();
        sel.map_symbol_to_test("parse_config", "fish-core");
        let out = sel.select(&["parse_config".to_string()], &[]);
        assert_eq!(out, vec!["fish-core"]);
    }

    #[test]
    fn changed_crate_selects_registered_targets() {
        let mut sel = TestSelector::new();
        sel.register_crate("fish-cache", &["cache-unit"]);
        let out = sel.select(&[], &["crates/fish-cache/src/gc.rs".to_string()]);
        assert_eq!(out, vec!["cache-unit"]);
    }

    #[test]
    fn windows_paths_normalized() {
        let mut sel = TestSelector::new();
        sel.register_crate("fish-cli", &["cli-unit"]);
        let out = sel.select(&[], &["crates\\fish-cli\\src\\main.rs".to_string()]);
        assert_eq!(out, vec!["cli-unit"]);
    }

    #[test]
    fn integration_test_file_maps_to_named_target() {
        let sel = TestSelector::new();
        let out = sel.select(&[], &["crates/fish-executor/tests/pipeline.rs".to_string()]);
        assert_eq!(out, vec!["test:pipeline"]);
    }

    #[test]
    fn integration_test_file_at_root_maps_to_named_target() {
        let sel = TestSelector::new();
        let out = sel.select(&[], &["tests/cli_tests.rs".to_string()]);
        assert_eq!(out, vec!["test:cli_tests"]);
    }

    #[test]
    fn results_are_deduplicated_and_ordered() {
        let mut sel = TestSelector::new();
        sel.map_symbol_to_test("a", "t2");
        sel.map_symbol_to_test("b", "t1");
        sel.map_symbol_to_test("c", "t1");
        let out = sel.select(&["a".into(), "b".into(), "c".into()], &[]);
        assert_eq!(out, vec!["t1", "t2"]);
    }

    #[test]
    fn skip_ratio_sane() {
        let sel = TestSelector::new();
        assert!((sel.skip_ratio(3, 10) - 0.7).abs() < 1e-9);
        assert_eq!(sel.skip_ratio(0, 0), 0.0);
    }
}
