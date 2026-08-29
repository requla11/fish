use banana::ast::{DependencyGraph, PolyglotAstEngine, SemanticSymbol};
use std::path::{Path, PathBuf};

pub struct FishAstService;

impl FishAstService {
    pub fn parse_file(file: &Path) -> Result<Vec<SemanticSymbol>, anyhow::Error> {
        let ext = file.extension().and_then(|s| s.to_str()).unwrap_or("");
        let content = std::fs::read_to_string(file)?;
        Ok(PolyglotAstEngine::extract_symbols(&content, ext))
    }

    pub fn validate_no_cycles(files: &[PathBuf]) -> Result<(), anyhow::Error> {
        let mut graph = DependencyGraph::new();
        for f in files {
            graph.nodes.insert(f.clone());
        }
        let cycles = graph.detect_cycles();
        if !cycles.is_empty() {
            anyhow::bail!("Detected {} circular dependency cycles", cycles.len());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_fish_ast_service_integration() {
        let temp = tempdir().unwrap();
        let rs_file = temp.path().join("main.rs");
        std::fs::write(
            &rs_file,
            b"pub fn run_build() {}\npub struct BuildConfig {}",
        )
        .unwrap();

        let symbols = FishAstService::parse_file(&rs_file).unwrap();
        assert_eq!(symbols.len(), 2);
        assert_eq!(symbols[0].name, "run_build");

        assert!(FishAstService::validate_no_cycles(&[rs_file]).is_ok());
    }
}
