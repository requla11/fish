use apple::protocol::{ExecutionRequest, ExecutionResult, SandboxProfile};
use apple::{AppleDaemonServer, DeterminismVerifier, VerificationReport};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub struct AppleBridge {
    server: AppleDaemonServer,
    scratch_dir: PathBuf,
}

impl AppleBridge {
    pub fn new(scratch_dir: impl Into<PathBuf>) -> Self {
        let path = scratch_dir.into();
        Self {
            server: AppleDaemonServer::new(path.clone()),
            scratch_dir: path,
        }
    }

    pub async fn execute_sandboxed(
        &self,
        task_id: impl Into<String>,
        working_dir: impl Into<PathBuf>,
        argv: Vec<String>,
        env: HashMap<String, String>,
        profile: Option<SandboxProfile>,
    ) -> ExecutionResult {
        let request = ExecutionRequest {
            task_id: task_id.into(),
            working_dir: working_dir.into(),
            argv,
            env,
            profile: profile.unwrap_or_default(),
        };
        self.server.execute_task(request).await
    }

    pub async fn verify_artifact_reproducibility(
        &self,
        task_id: impl Into<String>,
        working_dir: impl Into<PathBuf>,
        argv: Vec<String>,
        env: HashMap<String, String>,
        artifact_rel_path: &Path,
    ) -> Result<VerificationReport, anyhow::Error> {
        let verifier = DeterminismVerifier::new(self.scratch_dir.clone());
        let request = ExecutionRequest {
            task_id: task_id.into(),
            working_dir: working_dir.into(),
            argv,
            env,
            profile: SandboxProfile::default(),
        };
        verifier
            .verify_reproducible(request, artifact_rel_path)
            .await
    }
}
