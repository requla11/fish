//! Composition root for all ecosystem backends.
//!
//! Every backend crate exposes a stateless `*EcosystemBackend`
//! implementing `fish_backend_api::EcosystemBackend`. Registration
//! happens here and only here — adding an ecosystem is one line in
//! [`registry`] instead of a new match arm in every dispatcher.

use std::path::Path;
use std::sync::OnceLock;

use fish_backend_api::{BuildGraph, BuildMode, Ecosystem, EcosystemBackend};
use fish_executor::Task;

#[cfg(feature = "backend-cc")]
use fish_backend_cc::ecosystem::CcEcosystemBackend;
#[cfg(feature = "backend-dart")]
use fish_backend_dart::ecosystem::DartEcosystemBackend;
#[cfg(feature = "backend-docker")]
use fish_backend_docker::ecosystem::DockerEcosystemBackend;
#[cfg(feature = "backend-dotnet")]
use fish_backend_dotnet::ecosystem::DotnetEcosystemBackend;
#[cfg(feature = "backend-go")]
use fish_backend_go::ecosystem::GoEcosystemBackend;
#[cfg(feature = "backend-java")]
use fish_backend_java::ecosystem::JavaEcosystemBackend;
#[cfg(feature = "backend-py")]
use fish_backend_py::ecosystem::PyEcosystemBackend;
use fish_backend_rust::ecosystem::RustEcosystemBackend;
#[cfg(feature = "backend-swift")]
use fish_backend_swift::ecosystem::SwiftEcosystemBackend;
#[cfg(feature = "backend-ts")]
use fish_backend_ts::ecosystem::TsEcosystemBackend;
#[cfg(feature = "backend-zig")]
use fish_backend_zig::ecosystem::ZigEcosystemBackend;

#[allow(clippy::vec_init_then_push)]
fn instantiate() -> Vec<Box<dyn EcosystemBackend>> {
    let mut backends: Vec<Box<dyn EcosystemBackend>> = vec![Box::new(RustEcosystemBackend)];
    #[cfg(feature = "backend-ts")]
    backends.push(Box::new(TsEcosystemBackend));
    #[cfg(feature = "backend-py")]
    backends.push(Box::new(PyEcosystemBackend));
    #[cfg(feature = "backend-go")]
    backends.push(Box::new(GoEcosystemBackend));
    #[cfg(feature = "backend-cc")]
    backends.push(Box::new(CcEcosystemBackend));
    #[cfg(feature = "backend-java")]
    backends.push(Box::new(JavaEcosystemBackend));
    #[cfg(feature = "backend-dotnet")]
    backends.push(Box::new(DotnetEcosystemBackend));
    #[cfg(feature = "backend-swift")]
    backends.push(Box::new(SwiftEcosystemBackend));
    #[cfg(feature = "backend-dart")]
    backends.push(Box::new(DartEcosystemBackend));
    #[cfg(feature = "backend-zig")]
    backends.push(Box::new(ZigEcosystemBackend));
    #[cfg(feature = "backend-docker")]
    backends.push(Box::new(DockerEcosystemBackend));
    backends
}

/// All registered backends, in stable priority order.
pub fn registry() -> &'static [Box<dyn EcosystemBackend>] {
    static REGISTRY: OnceLock<Vec<Box<dyn EcosystemBackend>>> = OnceLock::new();
    REGISTRY.get_or_init(instantiate)
}

/// First backend claiming the given ecosystem.
pub fn for_ecosystem(ecosystem: Ecosystem) -> Option<&'static (dyn EcosystemBackend + 'static)> {
    registry()
        .iter()
        .find(|b| b.ecosystems().contains(&ecosystem))
        .map(|b| b.as_ref())
}

/// Build the task graph for `ecosystem` rooted at `dir`.
///
/// Unknown ecosystems and "nothing here" both yield an empty graph.
pub fn build_subgraph(
    ecosystem: Ecosystem,
    dir: &Path,
    mode: BuildMode,
) -> Result<BuildGraph<Task>, String> {
    match for_ecosystem(ecosystem) {
        Some(backend) if backend.detect(dir) => backend.build_task_graph(dir, mode),
        _ => Ok(BuildGraph::new()),
    }
}
