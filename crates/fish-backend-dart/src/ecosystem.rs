#![forbid(unsafe_code)]

//! Adapter exposing the Dart/Flutter backend through the uniform
//! [`EcosystemBackend`] contract.

use std::path::Path;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

use crate::{DartBackend, DartProjectConfig};

#[derive(Debug, Clone, Copy, Default)]
pub struct DartEcosystemBackend;

impl EcosystemBackend for DartEcosystemBackend {
    fn id(&self) -> &'static str {
        "dart"
    }

    fn ecosystems(&self) -> &'static [Ecosystem] {
        &[Ecosystem::Dart]
    }

    fn detect(&self, dir: &Path) -> bool {
        dir.join("pubspec.yaml").is_file()
    }

    fn build_task_graph(&self, dir: &Path, _mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        let config = DartProjectConfig::detect(dir).map_err(|e| e.to_string())?;
        let backend = DartBackend::new().map_err(|e| e.to_string())?;
        let build_dir = dir.join("build");
        backend
            .create_tasks_from_config(&config, dir, &build_dir)
            .map_err(|e| e.to_string())
    }
}
