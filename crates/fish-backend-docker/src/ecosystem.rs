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
        DockerBackend::detect_config(dir).is_some() && crate::DockerToolchain::detect().is_ok()
    }

    fn build_task_graph(&self, dir: &Path, _mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        match DockerBackend::detect_config(dir) {
            Some(config) => {
                let backend = match DockerBackend::new(config) {
                    Ok(b) => b,
                    Err(_) => return Ok(BuildGraph::new()),
                };
                backend.build_task_graph().map_err(|e| e.to_string())
            }
            None => Ok(BuildGraph::new()),
        }
    }
}
