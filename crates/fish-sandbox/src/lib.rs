#![forbid(unsafe_code)]

pub mod apple_bridge;
pub mod env;
pub mod executor;
pub mod file_events;
pub mod hermetic;
pub mod isolation;
pub mod microvm;
pub mod microvm_config;
pub mod snapshot;
pub mod tracer;

pub use apple_bridge::AppleBridge;
pub use env::{EnvPolicy, sanitize_env};
pub use executor::{SandboxConfig, SandboxedExecutor};
pub use file_events::{FileAccessType, FileEventRecorder, HermeticitySummary};
pub use hermetic::{HermeticProcessSandbox, SandboxPlatform};
pub use isolation::{FsPolicy, SandboxWorkspace};
pub use snapshot::{SandboxSnapshot, SnapshotBackend, SnapshotManager};
pub use tracer::{HermeticTraceResult, SyscallTracer};

#[cfg(test)]
mod tests {
    use super::*;
    use fish_executor::{CommandSpec, ProcessExecutor, Task, TaskExecutor};
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[test]
    fn test_hermetic_env_sanitization() {
        let mut extra = HashMap::new();
        extra.insert("CUSTOM_VAR".to_string(), "custom_val".to_string());

        let env = sanitize_env(&EnvPolicy::Hermetic, &extra);

        assert_eq!(env.get("LANG").map(|s| s.as_str()), Some("C"));
        assert_eq!(env.get("LC_ALL").map(|s| s.as_str()), Some("C"));
        assert_eq!(env.get("TZ").map(|s| s.as_str()), Some("UTC"));
        assert_eq!(
            env.get("CUSTOM_VAR").map(|s| s.as_str()),
            Some("custom_val")
        );
    }

    #[test]
    fn test_sandboxed_executor_runs_task() {
        let temp = tempdir().unwrap();
        let process = ProcessExecutor::new(false);
        let config = SandboxConfig::default();
        let executor = SandboxedExecutor::new(process, config);

        let prog = if cfg!(windows) { "cmd" } else { "sh" };
        let args = if cfg!(windows) {
            vec!["/C".to_string(), "exit 0".to_string()]
        } else {
            vec!["-c".to_string(), "exit 0".to_string()]
        };

        let spec = CommandSpec::new(prog).args(args).cwd(temp.path());
        let task = Task::new("test_task", spec.command_line(), spec);

        let outcome = executor.execute(&task).unwrap();
        assert_eq!(outcome.exit_code, Some(0));
    }

    #[tokio::test]
    async fn test_apple_bridge_executes_task_in_scratch() {
        let temp = tempdir().unwrap();
        let bridge = AppleBridge::new(temp.path().to_path_buf());

        let prog = if cfg!(windows) { "cmd" } else { "sh" };
        let argv = if cfg!(windows) {
            vec![prog.to_string(), "/C".to_string(), "echo ok".to_string()]
        } else {
            vec![prog.to_string(), "-c".to_string(), "echo ok".to_string()]
        };

        let res = bridge
            .execute_sandboxed(
                "task_001",
                temp.path().to_path_buf(),
                argv,
                HashMap::new(),
                None,
            )
            .await;

        assert_eq!(res.exit_code, 0);
        assert!(res.hermetic_guarantee);
    }

    #[tokio::test]
    async fn test_apple_bridge_executes_task_hermetic() {
        let temp = tempdir().unwrap();
        let bridge = AppleBridge::new(temp.path().to_path_buf());

        let prog = if cfg!(windows) { "cmd" } else { "sh" };
        let args = if cfg!(windows) {
            vec!["/C".to_string(), "echo hermetic".to_string()]
        } else {
            vec!["-c".to_string(), "echo hermetic".to_string()]
        };

        let spec = CommandSpec::new(prog).args(args).cwd(temp.path());
        let task = Task::new("hermetic_task", "hermetic description", spec)
            .with_inputs(vec![temp.path().join("input.txt")])
            .with_artifacts(vec![temp.path().join("output.bin")]);

        let res = bridge.execute_task_hermetic(&task, None).await;

        assert_eq!(res.exit_code, 0);
        assert_eq!(res.task_id, "hermetic_task");
    }
}
