use std::collections::HashMap;

use fish_executor::{CommandSpec, ExecutorError, Task, TaskExecutor, TaskOutcome};

use crate::env::{EnvPolicy, sanitize_env};
use crate::isolation::FsPolicy;

#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub env_policy: EnvPolicy,
    pub fs_policy: FsPolicy,
    pub extra_env: HashMap<String, String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            env_policy: EnvPolicy::Hermetic,
            fs_policy: FsPolicy::InPlace,
            extra_env: HashMap::new(),
        }
    }
}

pub struct SandboxedExecutor<E: TaskExecutor> {
    inner: E,
    config: SandboxConfig,
}

impl<E: TaskExecutor> SandboxedExecutor<E> {
    pub fn new(inner: E, config: SandboxConfig) -> Self {
        Self { inner, config }
    }

    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }

    pub fn inner(&self) -> &E {
        &self.inner
    }
}

impl<E: TaskExecutor + Sync> TaskExecutor for SandboxedExecutor<E> {
    fn execute(&self, task: &Task) -> Result<TaskOutcome, ExecutorError> {
        let env = sanitize_env(&self.config.env_policy, &self.config.extra_env);

        let original_cwd = task
            .spec
            .cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let workspace = match self.config.fs_policy {
            FsPolicy::InPlace => crate::isolation::SandboxWorkspace::in_place(original_cwd.clone()),
            FsPolicy::IsolatedTemp => {
                let ws =
                    crate::isolation::SandboxWorkspace::isolated(&original_cwd).map_err(|e| {
                        ExecutorError::SpawnFailed(format!(
                            "Failed to create isolated workspace: {}",
                            e
                        ))
                    })?;
                // Copy inputs
                for input in &task.inputs {
                    if input.exists() {
                        let _ = ws.copy_file(input, input);
                    }
                }
                ws
            }
        };

        let active_cwd = workspace.root().to_path_buf();

        // Wrap command using HermeticProcessSandbox
        use crate::hermetic::HermeticProcessSandbox;
        let sandbox =
            HermeticProcessSandbox::auto_configure(active_cwd.clone(), active_cwd.clone());
        let (wrapped_prog, wrapped_args) =
            sandbox.wrap_command_args(&task.spec.program, &task.spec.args);

        let mut sandboxed_spec = CommandSpec::new(&wrapped_prog)
            .args(wrapped_args)
            .cwd(&active_cwd);

        if self.config.env_policy != EnvPolicy::Inherit {
            sandboxed_spec = sandboxed_spec.env_clear();
        }
        for (k, v) in env {
            sandboxed_spec = sandboxed_spec.env(k, v);
        }

        let mut sandboxed_task = task.clone();
        sandboxed_task.spec = sandboxed_spec;

        let outcome = self.inner.execute(&sandboxed_task);

        // If isolated, copy artifacts back
        if self.config.fs_policy == FsPolicy::IsolatedTemp && outcome.is_ok() {
            for artifact in &task.artifacts {
                let src = active_cwd.join(artifact);
                let dest = original_cwd.join(artifact);
                if src.exists() {
                    if let Some(parent) = dest.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let _ = std::fs::copy(&src, &dest);
                }
            }
        }

        outcome
    }
}
