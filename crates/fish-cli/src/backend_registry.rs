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

use fish_backend_cc::ecosystem::CcEcosystemBackend;
use fish_backend_dart::ecosystem::DartEcosystemBackend;
use fish_backend_docker::ecosystem::DockerEcosystemBackend;
use fish_backend_dotnet::ecosystem::DotnetEcosystemBackend;
use fish_backend_go::ecosystem::GoEcosystemBackend;
use fish_backend_java::ecosystem::JavaEcosystemBackend;
use fish_backend_py::ecosystem::PyEcosystemBackend;
use fish_backend_rust::ecosystem::RustEcosystemBackend;
use fish_backend_swift::ecosystem::SwiftEcosystemBackend;
use fish_backend_ts::ecosystem::TsEcosystemBackend;
use fish_backend_zig::ecosystem::ZigEcosystemBackend;

fn instantiate() -> Vec<Box<dyn EcosystemBackend>> {
    vec![
        Box::new(RustEcosystemBackend),
        Box::new(TsEcosystemBackend),
        Box::new(PyEcosystemBackend),
        Box::new(GoEcosystemBackend),
        Box::new(CcEcosystemBackend),
        Box::new(JavaEcosystemBackend),
        Box::new(DotnetEcosystemBackend),
        Box::new(SwiftEcosystemBackend),
        Box::new(DartEcosystemBackend),
        Box::new(ZigEcosystemBackend),
        Box::new(DockerEcosystemBackend),
    ]
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
