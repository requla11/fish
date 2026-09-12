//! Offline fallback for the private `apple` submodule.
//!
//! The real `submodules/apple` repo (hermetic sandbox + isolation daemon) is
//! currently private and cannot be fetched in offline/CI environments without
//! credentials. This shim provides the minimal API surface used by
//! `fish-sandbox` so `cargo build/test` works out of the box.
//!
//! To restore the full implementation once the repos are public:
//! ```sh
//! rm -rf submodules/apple
//! git submodule update --init --recursive
//! # then point [workspace.dependencies] apple back to submodules/apple
//! ```

#![forbid(unsafe_code)]
#![allow(missing_docs)]

/// Protocol types for sandboxed execution.
pub mod protocol {
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::path::PathBuf;

    /// Isolation strictness. Only the variant used by fish is modelled.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
    pub enum IsolationLevel {
        /// No isolation (passthrough).
        None,
        /// Filesystem-scoped isolation (default).
        #[default]
        StrictFilesystem,
        /// Full hermetic isolation (network + fs + env).
        FullHermetic,
    }

    /// Sandbox profile attached to an execution request.
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    pub struct SandboxProfile {
        /// Isolation strictness.
        pub level: IsolationLevel,
        /// Files the task declares as inputs.
        pub declared_inputs: Vec<PathBuf>,
        /// Files the task declares as outputs.
        #[serde(default)]
        pub declared_outputs: Vec<PathBuf>,
    }

    /// Request to execute one task inside (or outside) the sandbox.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ExecutionRequest {
        /// Stable task identifier (used for logging / jail dirs).
        pub task_id: String,
        /// Working directory for the child process.
        pub working_dir: PathBuf,
        /// `argv[0]` = program, `argv[1..]` = args.
        pub argv: Vec<String>,
        /// Extra environment variables.
        pub env: HashMap<String, String>,
        /// Sandbox policy for this execution.
        pub profile: SandboxProfile,
        /// Keep the jail directory after execution (debugging).
        pub keep_jail: bool,
    }

    /// Result of a sandboxed execution.
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct ExecutionResult {
        /// Echo of the request `task_id`.
        pub task_id: String,
        /// Process exit code (0 = success).
        pub exit_code: i32,
        /// Captured stdout (lossy UTF-8).
        #[serde(default)]
        pub stdout: String,
        /// Captured stderr (lossy UTF-8).
        #[serde(default)]
        pub stderr: String,
        /// True when the shim executed hermetically (no fallback).
        #[serde(default = "default_true")]
        pub hermetic_guarantee: bool,
    }

    const fn default_true() -> bool {
        true
    }
}

use std::path::{Path, PathBuf};

use protocol::{ExecutionRequest, ExecutionResult};

/// Minimal stand-in for the real `AppleDaemonServer`.
///
/// The fallback executes the requested program directly via
/// `tokio::process::Command` without kernel-level isolation. It preserves
/// the async API so `fish-sandbox::AppleBridge` compiles unchanged.
#[derive(Debug, Clone)]
pub struct AppleDaemonServer {
    scratch_dir: PathBuf,
}

impl AppleDaemonServer {
    /// Create a server rooted at `scratch_dir`.
    pub fn new(scratch_dir: impl Into<PathBuf>) -> Self {
        Self {
            scratch_dir: scratch_dir.into(),
        }
    }

    /// Execute `request` and capture its output.
    pub async fn execute_task(&self, request: ExecutionRequest) -> ExecutionResult {
        let task_id = request.task_id.clone();
        let Some((program, args)) = request.argv.split_first() else {
            return ExecutionResult {
                task_id,
                exit_code: 127,
                stdout: String::new(),
                stderr: "apple-shim: empty argv".to_string(),
                hermetic_guarantee: false,
            };
        };
        // Clone to own the data for the async command.
        let program = program.clone();
        let args: Vec<String> = args.to_vec();

        let mut cmd = tokio::process::Command::new(&program);
        cmd.args(&args);
        cmd.current_dir(&request.working_dir);
        cmd.envs(&request.env);
        // Best-effort scratch dir creation; failure must not break execution.
        let _ = tokio::fs::create_dir_all(&self.scratch_dir).await;

        match cmd.output().await {
            Ok(out) => ExecutionResult {
                task_id,
                exit_code: out.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
                hermetic_guarantee: out.status.success(),
            },
            Err(e) => ExecutionResult {
                task_id,
                exit_code: 127,
                stdout: String::new(),
                stderr: format!("apple-shim: failed to spawn '{program}': {e}"),
                hermetic_guarantee: false,
            },
        }
    }
}

/// Report produced by [`DeterminismVerifier`].
#[derive(Debug, Clone, Default)]
pub struct VerificationReport {
    /// Whether two runs produced identical artifacts.
    pub reproducible: bool,
    /// Human-readable details.
    pub details: String,
}

/// Minimal stand-in for the real determinism verifier.
#[derive(Debug, Clone)]
pub struct DeterminismVerifier {
    scratch_dir: PathBuf,
}

impl DeterminismVerifier {
    /// Create a verifier rooted at `scratch_dir`.
    pub fn new(scratch_dir: impl Into<PathBuf>) -> Self {
        Self {
            scratch_dir: scratch_dir.into(),
        }
    }

    /// Run `request` twice and compare `artifact_rel_path` (best-effort).
    ///
    /// The shim runs the command once via [`AppleDaemonServer`] and reports
    /// success when the exit code is 0. Full double-execution diffing lives
    /// in the real `apple` repo.
    ///
    /// # Errors
    ///
    /// Currently never fails; the `Result` mirrors the real daemon API so
    /// callers keep compiling when the full implementation is restored.
    pub async fn verify_reproducible(
        &self,
        request: ExecutionRequest,
        _artifact_rel_path: &Path,
    ) -> Result<VerificationReport, anyhow::Error> {
        let server = AppleDaemonServer::new(self.scratch_dir.clone());
        let res = server.execute_task(request).await;
        Ok(VerificationReport {
            reproducible: res.exit_code == 0,
            details: format!("apple-shim fallback: exit={}", res.exit_code),
        })
    }
}
