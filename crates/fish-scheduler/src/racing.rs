use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use fish_executor::{ExecutorError, Task, TaskExecutor, TaskOutcome};

/// Race the same task on a local and a remote executor and keep whichever
/// outcome arrives first.
///
/// # Semantics
///
/// Both sides are started unless the race has already been decided, and
/// neither execution is aborted mid-flight: `TaskExecutor` offers no
/// cancellation hook, so the losing side simply has its result discarded.
/// Racing therefore duplicates work and doubles the side effects of the task;
/// only use it for tasks whose repeated execution is safe (for example,
/// cache-friendly compilations writing atomic outputs).
pub struct DynamicRacingExecutor<L, R> {
    local_executor: Arc<L>,
    remote_executor: Arc<R>,
    remote_grace_period: Duration,
}

impl<L, R> DynamicRacingExecutor<L, R>
where
    L: TaskExecutor + Send + Sync + 'static,
    R: TaskExecutor + Send + Sync + 'static,
{
    pub fn new(local: L, remote: R, remote_grace_period: Duration) -> Self {
        Self {
            local_executor: Arc::new(local),
            remote_executor: Arc::new(remote),
            remote_grace_period,
        }
    }

    pub fn execute_race(&self, task: &Task) -> Result<TaskOutcome, ExecutorError> {
        let (tx, rx) = crossbeam_channel::bounded::<Result<TaskOutcome, ExecutorError>>(2);
        let cancelled = Arc::new(AtomicBool::new(false));

        let local_exec = Arc::clone(&self.local_executor);
        let remote_exec = Arc::clone(&self.remote_executor);
        let task_local = task.clone();
        let task_remote = task.clone();
        let tx_local = tx.clone();
        let tx_remote = tx;
        let cancelled_local = Arc::clone(&cancelled);
        let cancelled_remote = Arc::clone(&cancelled);
        let grace = self.remote_grace_period;

        std::thread::spawn(move || {
            if cancelled_local.load(Ordering::SeqCst) {
                return;
            }
            let res = local_exec.execute(&task_local);
            if !cancelled_local.load(Ordering::SeqCst) {
                let _ = tx_local.send(res);
            }
        });

        std::thread::spawn(move || {
            if !grace.is_zero() {
                std::thread::sleep(grace);
            }
            if cancelled_remote.load(Ordering::SeqCst) {
                return;
            }
            let res = remote_exec.execute(&task_remote);
            if !cancelled_remote.load(Ordering::SeqCst) {
                let _ = tx_remote.send(res);
            }
        });

        match rx.recv() {
            Ok(outcome_res) => {
                cancelled.store(true, Ordering::SeqCst);
                outcome_res
            }
            Err(_) => Err(ExecutorError::Spawn {
                command: task.label.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::Interrupted,
                    "Dynamic racing executors terminated without outcome",
                ),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fish_executor::{CommandSpec, TaskStatus};
    use std::time::Instant;

    struct MockRacingExecutor {
        delay: Duration,
        tag: String,
        fail: bool,
    }

    impl MockRacingExecutor {
        fn success(delay: Duration, tag: &str) -> Self {
            Self {
                delay,
                tag: tag.to_string(),
                fail: false,
            }
        }

        fn failure(delay: Duration, tag: &str) -> Self {
            Self {
                delay,
                tag: tag.to_string(),
                fail: true,
            }
        }
    }

    impl TaskExecutor for MockRacingExecutor {
        fn execute(&self, task: &Task) -> Result<TaskOutcome, ExecutorError> {
            if !self.delay.is_zero() {
                std::thread::sleep(self.delay);
            }
            if self.fail {
                Ok(TaskOutcome::failed(task, &self.tag))
            } else {
                Ok(TaskOutcome {
                    status: TaskStatus::Executed,
                    exit_code: Some(0),
                    duration: self.delay,
                    stdout: self.tag.clone(),
                    stderr: String::new(),
                })
            }
        }
    }

    #[test]
    fn test_dynamic_racing_fast_local_wins() {
        let racer = DynamicRacingExecutor::new(
            MockRacingExecutor::success(Duration::from_millis(5), "local"),
            MockRacingExecutor::success(Duration::from_millis(150), "remote"),
            Duration::from_millis(10),
        );
        let spec = CommandSpec::new("echo");
        let task = Task::new("test", "echo", spec);
        let start = Instant::now();
        let outcome = racer.execute_race(&task).unwrap();
        assert_eq!(outcome.status, TaskStatus::Executed);
        assert_eq!(outcome.stdout, "local");
        assert!(start.elapsed() < Duration::from_millis(120));
    }

    #[test]
    fn test_dynamic_racing_fast_remote_wins() {
        let racer = DynamicRacingExecutor::new(
            MockRacingExecutor::success(Duration::from_millis(150), "local"),
            MockRacingExecutor::success(Duration::from_millis(5), "remote"),
            Duration::ZERO,
        );
        let spec = CommandSpec::new("echo");
        let task = Task::new("test", "echo", spec);
        let start = Instant::now();
        let outcome = racer.execute_race(&task).unwrap();
        assert_eq!(outcome.status, TaskStatus::Executed);
        assert_eq!(outcome.stdout, "remote");
        assert!(start.elapsed() < Duration::from_millis(120));
    }

    #[test]
    fn test_dynamic_racing_winner_failure_propagated() {
        let racer = DynamicRacingExecutor::new(
            MockRacingExecutor::failure(Duration::from_millis(5), "local-error"),
            MockRacingExecutor::success(Duration::from_millis(150), "remote"),
            Duration::from_millis(10),
        );
        let spec = CommandSpec::new("echo");
        let task = Task::new("test", "echo", spec);
        let outcome = racer.execute_race(&task).unwrap();
        assert_eq!(outcome.status, TaskStatus::Failed);
        assert_eq!(outcome.stderr, "local-error");
    }
}
