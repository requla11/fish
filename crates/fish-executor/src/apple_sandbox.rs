use crate::command::CommandSpec;
use crate::executor::ExecutorError;
use crate::middleware::TaskMiddleware;
use crate::task::Task;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppleSandboxConfig {
    pub enabled: bool,
    pub offline: bool,
    pub memory_limit_mb: Option<u64>,
    pub timeout_seconds: Option<u64>,
    pub socket_path: Option<String>,
    pub declared_inputs: Vec<PathBuf>,
    pub declared_outputs: Vec<PathBuf>,
    pub max_processes: Option<u32>,
    pub numa_node: Option<u32>,
}

impl Default for AppleSandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            offline: true,
            memory_limit_mb: Some(4096),
            timeout_seconds: Some(300),
            socket_path: None,
            declared_inputs: Vec::new(),
            declared_outputs: Vec::new(),
            max_processes: Some(512),
            numa_node: None,
        }
    }
}

pub struct AppleSandboxAdapter;

impl AppleSandboxAdapter {
    pub fn wrap_command(spec: &CommandSpec, config: &AppleSandboxConfig) -> CommandSpec {
        if !config.enabled {
            return spec.clone();
        }

        let mut apple_args = vec!["run".to_string()];

        if config.offline {
            apple_args.push("--offline".to_string());
        }

        if let Some(mem) = config.memory_limit_mb {
            apple_args.push("--memory-limit-mb".to_string());
            apple_args.push(mem.to_string());
        }

        if let Some(to) = config.timeout_seconds {
            apple_args.push("--timeout-seconds".to_string());
            apple_args.push(to.to_string());
        }

        if let Some(ref cwd) = spec.cwd {
            apple_args.push("--workdir".to_string());
            apple_args.push(cwd.display().to_string());
        }

        for input in &config.declared_inputs {
            apple_args.push("--input".to_string());
            apple_args.push(input.display().to_string());
        }

        for output in &config.declared_outputs {
            apple_args.push("--output".to_string());
            apple_args.push(output.display().to_string());
        }

        apple_args.push("--".to_string());
        apple_args.push(spec.program.clone());
        apple_args.extend(spec.args.clone());

        let mut wrapped = CommandSpec::new("apple");
        wrapped.args = apple_args;
        wrapped.env = spec.env.clone();
        wrapped.env_clear = spec.env_clear;
        wrapped.cwd = spec.cwd.clone();
        wrapped
    }
}

pub struct AppleSandboxMiddleware {
    config: AppleSandboxConfig,
}

impl AppleSandboxMiddleware {
    pub fn new(config: AppleSandboxConfig) -> Self {
        Self { config }
    }
}

impl TaskMiddleware for AppleSandboxMiddleware {
    fn pre_execute(&self, task: &mut Task) -> Result<(), ExecutorError> {
        let mut task_config = self.config.clone();
        for input in &task.inputs {
            if !task_config.declared_inputs.contains(input) {
                task_config.declared_inputs.push(input.clone());
            }
        }
        for artifact in &task.artifacts {
            if !task_config.declared_outputs.contains(artifact) {
                task_config.declared_outputs.push(artifact.clone());
            }
        }
        task.spec = AppleSandboxAdapter::wrap_command(&task.spec, &task_config);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apple_sandbox_adapter_wraps_command() {
        let spec = CommandSpec::new("rustc")
            .arg("main.rs")
            .cwd(PathBuf::from("/workspace"));

        let config = AppleSandboxConfig::default();
        let wrapped = AppleSandboxAdapter::wrap_command(&spec, &config);

        assert_eq!(wrapped.program, "apple");
        assert!(wrapped.args.contains(&"run".to_string()));
        assert!(wrapped.args.contains(&"--offline".to_string()));
        assert!(wrapped.args.contains(&"--memory-limit-mb".to_string()));
        assert!(wrapped.args.contains(&"4096".to_string()));
        assert!(wrapped.args.contains(&"rustc".to_string()));
        assert!(wrapped.args.contains(&"main.rs".to_string()));
    }

    #[test]
    fn test_apple_sandbox_disabled_returns_verbatim() {
        let spec = CommandSpec::new("gcc").arg("-O3");
        let config = AppleSandboxConfig {
            enabled: false,
            ..Default::default()
        };

        let wrapped = AppleSandboxAdapter::wrap_command(&spec, &config);
        assert_eq!(wrapped.program, "gcc");
        assert_eq!(wrapped.args, vec!["-O3".to_string()]);
    }

    #[test]
    fn test_apple_sandbox_middleware_transforms_task_spec() {
        let spec = CommandSpec::new("cargo").arg("build");
        let mut task = Task::new("cargo_build", "build project", spec);
        let middleware = AppleSandboxMiddleware::new(AppleSandboxConfig::default());

        middleware.pre_execute(&mut task).unwrap();

        assert_eq!(task.spec.program, "apple");
        assert!(task.spec.args.contains(&"cargo".to_string()));
        assert!(task.spec.args.contains(&"build".to_string()));
    }

    #[test]
    fn test_apple_sandbox_middleware_propagates_inputs_and_outputs() {
        let spec = CommandSpec::new("rustc").arg("src/lib.rs");
        let task = Task::new("rustc_lib", "compile library", spec)
            .with_inputs(vec![
                PathBuf::from("src/lib.rs"),
                PathBuf::from("Cargo.toml"),
            ])
            .with_artifacts(vec![PathBuf::from("target/libfoo.rlib")]);

        let middleware = AppleSandboxMiddleware::new(AppleSandboxConfig::default());
        let mut transformed = task.clone();
        middleware.pre_execute(&mut transformed).unwrap();

        assert_eq!(transformed.spec.program, "apple");
        let args = &transformed.spec.args;
        assert!(args.iter().any(|a| a == "--input"));
        assert!(
            args.iter()
                .any(|a| a.contains("src/lib.rs") || a.contains("src\\lib.rs"))
        );
        assert!(args.iter().any(|a| a == "--output"));
        assert!(
            args.iter()
                .any(|a| a.contains("target/libfoo.rlib") || a.contains("target\\libfoo.rlib"))
        );
    }
}
