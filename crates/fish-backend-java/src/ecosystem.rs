#![forbid(unsafe_code)]

//! Adapter exposing the Java/Kotlin backend through the uniform
//! [`EcosystemBackend`] contract.

use std::path::Path;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

use crate::{JavaBackend, JavaProjectConfig};

const MANIFESTS: [&str; 5] = [
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
];

#[derive(Debug, Clone, Copy, Default)]
pub struct JavaEcosystemBackend;

impl EcosystemBackend for JavaEcosystemBackend {
    fn id(&self) -> &'static str {
        "java"
    }

    fn ecosystems(&self) -> &'static [Ecosystem] {
        &[Ecosystem::Java]
    }

    fn detect(&self, dir: &Path) -> bool {
        MANIFESTS.iter().any(|m| dir.join(m).is_file())
    }

    fn build_task_graph(&self, dir: &Path, _mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        let config = JavaProjectConfig::detect(dir).map_err(|e| e.to_string())?;
        let backend = JavaBackend::new().map_err(|e| e.to_string())?;
        let build_dir = dir.join("build");
        backend
            .create_tasks_from_config(&config, dir, &build_dir)
            .map_err(|e| e.to_string())
    }
}
