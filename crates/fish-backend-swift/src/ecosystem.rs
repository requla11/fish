#![forbid(unsafe_code)]

//! Adapter exposing the Swift backend through the uniform
//! [`EcosystemBackend`] contract.

use std::path::Path;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

use crate::{SwiftBackend, SwiftProjectConfig};

#[derive(Debug, Clone, Copy, Default)]
pub struct SwiftEcosystemBackend;

impl EcosystemBackend for SwiftEcosystemBackend {
    fn id(&self) -> &'static str {
        "swift"
    }

    fn ecosystems(&self) -> &'static [Ecosystem] {
        &[Ecosystem::Swift]
    }

    fn detect(&self, dir: &Path) -> bool {
        dir.join("Package.swift").is_file() || has_xcodeproj(dir)
    }

    fn build_task_graph(&self, dir: &Path, _mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        let config = SwiftProjectConfig::detect(dir).map_err(|e| e.to_string())?;
        let backend = SwiftBackend::new().map_err(|e| e.to_string())?;
        let build_dir = dir.join("build");
        backend
            .create_tasks_from_config(&config, dir, &build_dir)
            .map_err(|e| e.to_string())
    }
}

fn has_xcodeproj(dir: &Path) -> bool {
    std::fs::read_dir(dir).is_ok_and(|entries| {
        entries
            .flatten()
            .any(|e| e.path().extension().is_some_and(|ext| ext == "xcodeproj"))
    })
}
