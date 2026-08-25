#![forbid(unsafe_code)]

//! Adapter exposing the TypeScript/Node backend through the uniform
//! [`EcosystemBackend`] contract.

use std::path::Path;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

use crate::{TsBackend, TsProjectConfig};

#[derive(Debug, Clone, Copy, Default)]
pub struct TsEcosystemBackend;

impl EcosystemBackend for TsEcosystemBackend {
    fn id(&self) -> &'static str {
        "typescript"
    }

    fn ecosystems(&self) -> &'static [Ecosystem] {
        &[Ecosystem::TypeScript]
    }

    fn detect(&self, dir: &Path) -> bool {
        dir.join("package.json").is_file()
    }

    fn build_task_graph(&self, dir: &Path, _mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        let config = TsProjectConfig::discover_or_default(dir).map_err(|e| e.to_string())?;
        TsBackend::new()
            .build_task_graph(&config, dir)
            .map_err(|e| e.to_string())
    }
}
