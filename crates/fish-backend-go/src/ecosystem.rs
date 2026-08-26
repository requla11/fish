#![forbid(unsafe_code)]

//! Adapter exposing the Go backend through the uniform
//! [`EcosystemBackend`] contract, including the default-config
//! synthesis previously hard-coded in the CLI's polyglot dispatcher.

use std::path::Path;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

use crate::{GoBackend, GoProjectConfig};

#[derive(Debug, Clone, Copy, Default)]
pub struct GoEcosystemBackend;

impl GoEcosystemBackend {
    fn load_config(dir: &Path) -> Result<GoProjectConfig, String> {
        let config_path = dir.join("fish.go.json");
        if config_path.exists() {
            return GoProjectConfig::from_file(&config_path).map_err(|e| e.to_string());
        }

        let name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("app")
            .to_string();
        Ok(GoProjectConfig {
            name,
            package_path: "./...".to_string(),
            tags: vec![],
            ldflags: None,
            gcflags: None,
            run_tests: true,
            race: false,
            coverage: false,
            run_benchmarks: false,
            run_linter: true,
            output_binary: None,
            env: std::collections::HashMap::new(),
        })
    }
}

impl EcosystemBackend for GoEcosystemBackend {
    fn id(&self) -> &'static str {
        "go"
    }

    fn ecosystems(&self) -> &'static [Ecosystem] {
        &[Ecosystem::Go]
    }

    fn detect(&self, dir: &Path) -> bool {
        dir.join("go.mod").is_file()
    }

    fn build_task_graph(&self, dir: &Path, _mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        let config = Self::load_config(dir)?;
        let backend = GoBackend::new().map_err(|e| e.to_string())?;
        let build_dir = dir.join("build");
        backend
            .create_tasks_from_config(&config, dir, &build_dir)
            .map_err(|e| e.to_string())
    }
}
