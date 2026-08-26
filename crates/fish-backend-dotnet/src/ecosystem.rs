#![forbid(unsafe_code)]

//! Adapter exposing the .NET backend through the uniform
//! [`EcosystemBackend`] contract.

use std::path::Path;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

use crate::{DotnetBackend, DotnetProjectConfig};

#[derive(Debug, Clone, Copy, Default)]
pub struct DotnetEcosystemBackend;

impl EcosystemBackend for DotnetEcosystemBackend {
    fn id(&self) -> &'static str {
        "dotnet"
    }

    fn ecosystems(&self) -> &'static [Ecosystem] {
        &[Ecosystem::DotNet]
    }

    fn detect(&self, dir: &Path) -> bool {
        ["sln", "csproj"]
            .iter()
            .any(|ext| find_with_ext(dir, ext).is_some())
    }

    fn build_task_graph(&self, dir: &Path, _mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        let config = DotnetProjectConfig::detect(dir).map_err(|e| e.to_string())?;
        let backend = DotnetBackend::new().map_err(|e| e.to_string())?;
        let build_dir = dir.join("build");
        backend
            .create_tasks_from_config(&config, dir, &build_dir)
            .map_err(|e| e.to_string())
    }
}

fn find_with_ext(dir: &Path, extension: &str) -> Option<std::path::PathBuf> {
    std::fs::read_dir(dir).ok()?.flatten().find_map(|entry| {
        let path = entry.path();
        (path.extension().and_then(|s| s.to_str()) == Some(extension)).then_some(path)
    })
}
