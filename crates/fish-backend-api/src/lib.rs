#![forbid(unsafe_code)]

//! Contract every language backend implements so the rest of Fish can
//! stay ecosystem-agnostic.
//!
//! Historically each backend exposed a different entry point
//! (`create_tasks`, `build_task_graph`, `create_tasks_from_config`, …)
//! and `fish-cli` hard-coded one match arm per ecosystem. This crate
//! defines the uniform surface: a backend detects whether it owns a
//! directory and turns that directory into a task graph.

use std::path::Path;

use fish_executor::Task;
pub use fish_graph::BuildGraph;

/// What kind of build invocation is being graphed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildMode {
    Build,
    Check,
    Test,
    Clippy,
    Doc,
    Bench,
}

impl BuildMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Check => "check",
            Self::Test => "test",
            Self::Clippy => "clippy",
            Self::Doc => "doc",
            Self::Bench => "bench",
        }
    }

    /// Cargo subcommand equivalent, used by the Rust backend and CLI.
    pub fn cargo_subcommand(&self) -> &'static str {
        self.as_str()
    }
}

/// Ecosystems Fish can orchestrate, independent of any backend's own
/// config types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ecosystem {
    Rust,
    TypeScript,
    Go,
    Python,
    Cpp,
    Java,
    DotNet,
    Swift,
    Dart,
    Zig,
    Docker,
}

/// Uniform backend contract.
///
/// Implementations are expected to be cheap to construct (toolchain
/// probing is memoized inside each backend) and stateless between
/// calls: all per-project state lives in the target directory.
pub trait EcosystemBackend: Send + Sync {
    /// Stable identifier, e.g. `"rust"` — used in logs and cache keys.
    fn id(&self) -> &'static str;

    /// Every ecosystem this backend can handle.
    fn ecosystems(&self) -> &'static [Ecosystem];

    /// Does a buildable project of this ecosystem exist at `dir`?
    ///
    /// Must be side-effect free; `build_task_graph` re-resolves
    /// configuration so detect stays a cheap existence probe.
    fn detect(&self, dir: &Path) -> bool;

    /// Produce the task graph for the project rooted at `dir`.
    ///
    /// Returns an error string when the directory looks like this
    /// ecosystem but is misconfigured. Backends that find nothing to
    /// do return an empty graph rather than an error.
    fn build_task_graph(&self, dir: &Path, mode: BuildMode) -> Result<BuildGraph<Task>, String>;
}
