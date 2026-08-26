#![forbid(unsafe_code)]

//! Adapter exposing the C/C++ backend through the uniform
//! [`EcosystemBackend`] contract, including the source-scan fallback
//! previously hard-coded in the CLI's polyglot dispatcher.

use std::path::Path;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

use crate::{CcBackend, CcLanguage, CcOutputType, CcProjectConfig};

const CC_EXTS: [&str; 4] = ["cpp", "cc", "cxx", "c"];

#[derive(Debug, Clone, Copy, Default)]
pub struct CcEcosystemBackend;

fn scan_sources(root: &Path, prefix: &str) -> Vec<String> {
    let mut sources = Vec::new();
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let p = entry.path();
            let is_cc = p
                .extension()
                .is_some_and(|e| CC_EXTS.contains(&e.to_str().unwrap_or_default()));
            if is_cc && let Some(n) = p.file_name().and_then(|n| n.to_str()) {
                sources.push(format!("{prefix}{n}"));
            }
        }
    }
    sources
}

impl CcEcosystemBackend {
    fn load_config(dir: &Path) -> Result<CcProjectConfig, String> {
        let config_path = dir.join("fish.cc.json");
        if config_path.exists() {
            return CcProjectConfig::from_file(&config_path).map_err(|e| e.to_string());
        }

        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("app")
            .to_string();
        let mut sources = scan_sources(dir, "");
        if sources.is_empty() && dir.join("src").exists() {
            sources = scan_sources(&dir.join("src"), "src/");
        }

        Ok(CcProjectConfig {
            name,
            language: CcLanguage::Cpp,
            sources,
            includes: vec!["include".to_string()],
            cflags: vec![],
            cxxflags: vec![],
            ldflags: vec![],
            output_type: CcOutputType::Executable,
        })
    }
}

impl EcosystemBackend for CcEcosystemBackend {
    fn id(&self) -> &'static str {
        "cc"
    }

    fn ecosystems(&self) -> &'static [Ecosystem] {
        &[Ecosystem::Cpp]
    }

    fn detect(&self, dir: &Path) -> bool {
        if dir.join("fish.cc.json").is_file() {
            return true;
        }
        !scan_sources(dir, "").is_empty()
            || (dir.join("src").is_dir() && !scan_sources(&dir.join("src"), "").is_empty())
    }

    fn build_task_graph(&self, dir: &Path, _mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        let config = Self::load_config(dir)?;
        let backend = CcBackend::new(config.language).map_err(|e| e.to_string())?;
        let build_dir = dir.join("build");
        backend
            .create_tasks_from_config(&config, dir, &build_dir)
            .map_err(|e| e.to_string())
    }
}
