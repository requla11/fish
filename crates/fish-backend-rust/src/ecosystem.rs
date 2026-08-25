#![forbid(unsafe_code)]

//! Adapter exposing the Cargo workspace backend through the uniform
//! [`EcosystemBackend`] contract.

use std::path::Path;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

use crate::RustBackend;

/// Stateless handle; the heavy `RustBackend` is constructed lazily per
/// call and its toolchain probing is memoized process-wide.
#[derive(Debug, Clone, Copy, Default)]
pub struct RustEcosystemBackend;

impl EcosystemBackend for RustEcosystemBackend {
    fn id(&self) -> &'static str {
        "rust"
    }

    fn ecosystems(&self) -> &'static [Ecosystem] {
        &[Ecosystem::Rust]
    }

    fn detect(&self, dir: &Path) -> bool {
        dir.join("Cargo.toml").is_file()
    }

    fn build_task_graph(&self, dir: &Path, mode: BuildMode) -> Result<BuildGraph<Task>, String> {
        let Some(project) =
            fish_core::project::Project::discover(dir).map_err(|e| e.to_string())?
        else {
            // Detect() should have filtered this out; be lenient anyway.
            return Ok(BuildGraph::new());
        };

        let backend = RustBackend::new().map_err(|e| e.to_string())?;
        let package_graph = if mode == BuildMode::Test {
            project.build_test_graph()
        } else {
            project.build_graph()
        }
        .map_err(|e| e.to_string())?;

        backend
            .create_tasks(&project, &package_graph, mode, true)
            .map_err(|e| e.to_string())
    }
}
