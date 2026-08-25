#![forbid(unsafe_code)]

//! Adapter exposing the Docker backend through the uniform
//! [`EcosystemBackend`] contract.

use std::path::Path;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

use crate::DockerBackend;

#[derive(Debug, Clone, Copy, Default)]
pub struct DockerEcosystemBackend;

impl EcosystemBackend for DockerEcosystemBackend {
    fn id(&self) -> &'static str {
        "docker"
    }

    fn ecosystems(&self) -> &'static [Ecosystem] {
        &[Ecosystem::Docker]
    }

    fn detect(&self, dir: &Path) -> bool {
        DockerBackend::detect_config(dir).is_some()
    }

    fn build_task_graph(&self, dir: &Path, _mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        match DockerBackend::detect_config(dir) {
            Some(config) => {
                let backend = DockerBackend::new(config).map_err(|e| e.to_string())?;
                backend.build_task_graph().map_err(|e| e.to_string())
            }
            None => Ok(BuildGraph::new()),
        }
    }
}
