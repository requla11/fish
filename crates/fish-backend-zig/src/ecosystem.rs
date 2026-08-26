#![forbid(unsafe_code)]

//! Adapter exposing the Zig backend through the uniform
//! [`EcosystemBackend`] contract.

use std::path::Path;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

use crate::{ZigBackend, ZigProjectConfig};

#[derive(Debug, Clone, Copy, Default)]
pub struct ZigEcosystemBackend;

impl EcosystemBackend for ZigEcosystemBackend {
    fn id(&self) -> &'static str {
        "zig"
    }

    fn ecosystems(&self) -> &'static [Ecosystem] {
        &[Ecosystem::Zig]
    }

    fn detect(&self, dir: &Path) -> bool {
        dir.join("build.zig").is_file()
    }

    fn build_task_graph(&self, dir: &Path, _mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        let config = ZigProjectConfig::detect(dir).map_err(|e| e.to_string())?;
        let backend = ZigBackend::new().map_err(|e| e.to_string())?;
        let build_dir = dir.join("build");
        backend
            .create_tasks_from_config(&config, dir, &build_dir)
            .map_err(|e| e.to_string())
    }
}
