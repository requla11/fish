#![forbid(unsafe_code)]

use crate::executor::{ExecutorError, TaskExecutor};
use crate::task::{Task, TaskOutcome};

pub trait TaskMiddleware: Send + Sync {
    fn pre_execute(&self, _task: &mut Task) -> Result<(), ExecutorError> {
        Ok(())
    }

    fn post_execute(&self, _task: &Task, _outcome: &mut TaskOutcome) -> Result<(), ExecutorError> {
        Ok(())
    }
}

pub struct MiddlewareChainExecutor<E> {
    inner: E,
    middlewares: Vec<Box<dyn TaskMiddleware>>,
}

impl<E: TaskExecutor> MiddlewareChainExecutor<E> {
    pub fn new(inner: E) -> Self {
        Self {
            inner,
            middlewares: Vec::new(),
        }
    }

    pub fn with_middleware(mut self, middleware: Box<dyn TaskMiddleware>) -> Self {
        self.middlewares.push(middleware);
        self
    }
}

impl<E: TaskExecutor> TaskExecutor for MiddlewareChainExecutor<E> {
    fn execute(&self, task: &Task) -> Result<TaskOutcome, ExecutorError> {
        let mut modified_task = task.clone();
        for m in &self.middlewares {
            m.pre_execute(&mut modified_task)?;
        }

        let mut outcome = self.inner.execute(&modified_task)?;

        for m in self.middlewares.iter().rev() {
            m.post_execute(&modified_task, &mut outcome)?;
        }

        Ok(outcome)
    }
}

#[derive(Debug, Clone)]
pub struct NativeShimMiddleware {
    shim_path: Option<std::path::PathBuf>,
    log_dir: std::path::PathBuf,
}

impl NativeShimMiddleware {
    pub fn new(log_dir: impl Into<std::path::PathBuf>) -> Self {
        Self {
            shim_path: Self::discover_default_shim(),
            log_dir: log_dir.into(),
        }
    }

    pub fn with_shim_path(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.shim_path = Some(path.into());
        self
    }

    pub fn shim_path(&self) -> Option<&std::path::Path> {
        self.shim_path.as_deref()
    }

    pub fn log_dir(&self) -> &std::path::Path {
        &self.log_dir
    }

    pub fn discover_default_shim() -> Option<std::path::PathBuf> {
        if let Ok(path_str) = std::env::var("FISH_SHIM_PATH") {
            let p = std::path::PathBuf::from(path_str);
            if p.exists() {
                return Some(p);
            }
        }
        let candidates = [
            "fish_shim.dll",
            "libfish_shim.so",
            "libfish_shim.dylib",
            "cpp/build/bin/fish_shim.dll",
            "cpp/build/bin/libfish_shim.so",
            "cpp/build/bin/Release/fish_shim.dll",
        ];
        for candidate in candidates {
            let p = std::path::PathBuf::from(candidate);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }
}

impl TaskMiddleware for NativeShimMiddleware {
    fn pre_execute(&self, task: &mut Task) -> Result<(), ExecutorError> {
        let safe_label = task.label.replace([':', '/', '\\'], "_");
        let log_path = self.log_dir.join(format!("shim_trace_{}.log", safe_label));
        task.spec.env.insert(
            "FISH_SHIM_LOG".to_string(),
            log_path.to_string_lossy().to_string(),
        );

        if let Some(ref shim) = self.shim_path {
            let shim_str = shim.to_string_lossy().to_string();
            if cfg!(target_os = "linux") {
                task.spec
                    .env
                    .insert("LD_PRELOAD".to_string(), shim_str.clone());
            } else if cfg!(target_os = "macos") {
                task.spec
                    .env
                    .insert("DYLD_INSERT_LIBRARIES".to_string(), shim_str.clone());
            } else if cfg!(target_os = "windows") {
                task.spec.env.insert("FISH_SHIM_PATH".to_string(), shim_str);
            }
        }
        Ok(())
    }

    fn post_execute(&self, _task: &Task, _outcome: &mut TaskOutcome) -> Result<(), ExecutorError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CommandSpec;
    use crate::task::TaskStatus;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    struct DummyExecutor;
    impl TaskExecutor for DummyExecutor {
        fn execute(&self, task: &Task) -> Result<TaskOutcome, ExecutorError> {
            Ok(TaskOutcome {
                status: TaskStatus::Executed,
                exit_code: Some(0),
                stdout: format!("ran {}", task.label),
                stderr: String::new(),
                duration: Duration::from_millis(10),
            })
        }
    }

    struct FlagMiddleware {
        pre_called: Arc<AtomicBool>,
        post_called: Arc<AtomicBool>,
    }

    impl TaskMiddleware for FlagMiddleware {
        fn pre_execute(&self, _task: &mut Task) -> Result<(), ExecutorError> {
            self.pre_called.store(true, Ordering::SeqCst);
            Ok(())
        }

        fn post_execute(
            &self,
            _task: &Task,
            _outcome: &mut TaskOutcome,
        ) -> Result<(), ExecutorError> {
            self.post_called.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[test]
    fn test_middleware_chain_execution() {
        let pre_flag = Arc::new(AtomicBool::new(false));
        let post_flag = Arc::new(AtomicBool::new(false));

        let middleware = FlagMiddleware {
            pre_called: Arc::clone(&pre_flag),
            post_called: Arc::clone(&post_flag),
        };

        let executor =
            MiddlewareChainExecutor::new(DummyExecutor).with_middleware(Box::new(middleware));

        let mut spec = CommandSpec::new("echo");
        spec.args.push("hello".to_string());
        let task = Task::new("build_test", "test task", spec);
        let outcome = executor.execute(&task).unwrap();

        assert_eq!(outcome.status, TaskStatus::Executed);
        assert!(pre_flag.load(Ordering::SeqCst));
        assert!(post_flag.load(Ordering::SeqCst));
    }

    struct LoggingMiddleware {
        name: &'static str,
        log: Arc<std::sync::Mutex<Vec<String>>>,
        fail_pre: bool,
    }

    impl TaskMiddleware for LoggingMiddleware {
        fn pre_execute(&self, _task: &mut Task) -> Result<(), ExecutorError> {
            self.log.lock().unwrap().push(format!("{}:pre", self.name));
            if self.fail_pre {
                return Err(ExecutorError::Record {
                    command: self.name.to_string(),
                    source: std::io::Error::other("pre-execution failed"),
                });
            }
            Ok(())
        }

        fn post_execute(
            &self,
            _task: &Task,
            _outcome: &mut TaskOutcome,
        ) -> Result<(), ExecutorError> {
            self.log.lock().unwrap().push(format!("{}:post", self.name));
            Ok(())
        }
    }

    #[test]
    fn test_middleware_onion_ordering() {
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mw_a = LoggingMiddleware {
            name: "A",
            log: Arc::clone(&log),
            fail_pre: false,
        };
        let mw_b = LoggingMiddleware {
            name: "B",
            log: Arc::clone(&log),
            fail_pre: false,
        };

        let executor = MiddlewareChainExecutor::new(DummyExecutor)
            .with_middleware(Box::new(mw_a))
            .with_middleware(Box::new(mw_b));

        let spec = CommandSpec::new("echo");
        let task = Task::new("onion_test", "onion", spec);
        let outcome = executor.execute(&task).unwrap();
        assert_eq!(outcome.status, TaskStatus::Executed);

        let entries = log.lock().unwrap().clone();
        assert_eq!(entries, vec!["A:pre", "B:pre", "B:post", "A:post"]);
    }

    #[test]
    fn test_middleware_pre_execute_failure_aborts_chain() {
        let log = Arc::new(std::sync::Mutex::new(Vec::new()));
        let mw_a = LoggingMiddleware {
            name: "A",
            log: Arc::clone(&log),
            fail_pre: true,
        };
        let mw_b = LoggingMiddleware {
            name: "B",
            log: Arc::clone(&log),
            fail_pre: false,
        };

        let executor = MiddlewareChainExecutor::new(DummyExecutor)
            .with_middleware(Box::new(mw_a))
            .with_middleware(Box::new(mw_b));

        let spec = CommandSpec::new("echo");
        let task = Task::new("fail_test", "fail", spec);
        let err = executor.execute(&task).unwrap_err();
        match err {
            ExecutorError::Record { command, .. } => {
                assert_eq!(command, "A");
            }
            other => panic!("expected ExecutorError::Record, got {other:?}"),
        }

        let entries = log.lock().unwrap().clone();
        assert_eq!(entries, vec!["A:pre"]);
    }

    #[test]
    fn test_native_shim_middleware() {
        let temp = tempfile::tempdir().unwrap();
        let dummy_shim = temp.path().join("dummy_shim.dll");
        std::fs::write(&dummy_shim, b"").unwrap();

        let middleware = NativeShimMiddleware::new(temp.path()).with_shim_path(&dummy_shim);
        assert_eq!(middleware.shim_path(), Some(dummy_shim.as_path()));
        assert_eq!(middleware.log_dir(), temp.path());

        let spec = CommandSpec::new("echo");
        let mut task = Task::new("shim_test", "shim task", spec);
        middleware.pre_execute(&mut task).unwrap();

        assert!(task.spec.env.contains_key("FISH_SHIM_LOG"));
        if cfg!(windows) {
            assert_eq!(
                task.spec.env.get("FISH_SHIM_PATH"),
                Some(&dummy_shim.to_string_lossy().to_string())
            );
        } else if cfg!(target_os = "linux") {
            assert_eq!(
                task.spec.env.get("LD_PRELOAD"),
                Some(&dummy_shim.to_string_lossy().to_string())
            );
        }

        let mut outcome = TaskOutcome {
            status: TaskStatus::Executed,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            duration: Duration::from_millis(5),
        };
        assert!(middleware.post_execute(&task, &mut outcome).is_ok());
    }
}
