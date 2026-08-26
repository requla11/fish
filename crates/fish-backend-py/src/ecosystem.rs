#![forbid(unsafe_code)]

//! Adapter exposing the Python backend through the uniform
//! [`EcosystemBackend`] contract.

use std::path::Path;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

use crate::{PyBackend, PyProjectConfig};

#[derive(Debug, Clone, Copy, Default)]
pub struct PyEcosystemBackend;

impl EcosystemBackend for PyEcosystemBackend {
    fn id(&self) -> &'static str {
        "python"
    }

    fn ecosystems(&self) -> &'static [Ecosystem] {
        &[Ecosystem::Python]
    }

    fn detect(&self, dir: &Path) -> bool {
        ["pyproject.toml", "requirements.txt", "setup.py"]
            .iter()
            .any(|m| dir.join(m).is_file())
    }

    fn build_task_graph(&self, dir: &Path, _mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        let config = PyProjectConfig::discover_or_default(dir).map_err(|e| e.to_string())?;
        PyBackend::new()
            .build_task_graph(&config, dir)
            .map_err(|e| e.to_string())
    }
}
