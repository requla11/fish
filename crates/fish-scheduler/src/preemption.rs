use std::sync::Arc;
use std::time::Duration;

use fish_executor::{ExecutorError, Task, TaskExecutor, TaskOutcome};

/// Failure markers that indicate a *transient infrastructure loss* rather
/// than a genuine task failure. Spot/preemptible workers die with these
/// signatures; retrying them on surviving capacity is safe because the task
/// never produced results.
const DEFAULT_PREEMPTION_MARKERS: [&str; 6] = [
    "preempted",
    "spot interruption",
    "worker lost",
    "connection refused",
    "connection reset",
    "task lost",
];

fn is_preemption_failure(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    DEFAULT_PREEMPTION_MARKERS.iter().any(|m| lower.contains(m))
}

/// Wraps a preemptible (spot) primary executor with a fault-tolerant retry
/// policy and an on-demand fallback.
///
/// Only infrastructure-shaped failures are retried — a compile error from
/// the primary is returned immediately, unchanged. After `max_retries`
/// exhausted attempts against the primary, the task migrates to the fallback
/// executor once; if that also fails its real outcome is returned so callers
/// always see an honest diagnosis instead of a synthetic success.
pub struct PreemptionRetryExecutor<P, F> {
    primary: Arc<P>,
    fallback: Arc<F>,
    max_retries: usize,
    backoff: Duration,
}

impl<P, F> PreemptionRetryExecutor<P, F>
where
    P: TaskExecutor + Send + Sync + 'static,
    F: TaskExecutor + Send + Sync + 'static,
{
    pub fn new(primary: Arc<P>, fallback: Arc<F>, max_retries: usize, backoff: Duration) -> Self {
        Self {
            primary,
            fallback,
            max_retries: max_retries.max(1),
            backoff,
        }
    }
}

impl<P, F> TaskExecutor for PreemptionRetryExecutor<P, F>
where
    P: TaskExecutor + Send + Sync + 'static,
    F: TaskExecutor + Send + Sync + 'static,
{
    fn execute(&self, task: &Task) -> Result<TaskOutcome, ExecutorError> {
        let mut last_preempted_outcome: Option<TaskOutcome> = None;
        let mut last_preempted_err: Option<ExecutorError> = None;

        for attempt in 0..self.max_retries {
            if attempt > 0 && !self.backoff.is_zero() {
                std::thread::sleep(self.backoff);
            }

            match self.primary.execute(task) {
                Ok(outcome) if outcome.status == fish_executor::TaskStatus::Failed => {
                    if is_preemption_failure(&outcome.stderr)
                        || is_preemption_failure(&outcome.stdout)
                    {
                        last_preempted_outcome = Some(outcome);
                        continue;
                    }
                    // Genuine task failure is not ours to retry.
                    return Ok(outcome);
                }
                Ok(outcome) => return Ok(outcome),
                Err(err) => {
                    if is_preemption_failure(&err.to_string()) {
                        last_preempted_err = Some(err);
                        continue;
                    }
                    return Err(err);
                }
            }
        }

        // Primary capacity is gone for good this round: migrate to fallback.
        // If the fallback also fails, the primary's own diagnosis wins so no
        // evidence is replaced by a secondary symptom.
        match self.fallback.execute(task) {
            ok @ Ok(_) => ok,
            // The fallback's own symptom is intentionally dropped when the
            // primary already produced a real diagnosis.
            Err(_fallback_err) => {
                if let Some(err) = last_preempted_err {
                    return Err(err);
                }
                Ok(last_preempted_outcome
                    .expect("retries ran, so an outcome or error was captured"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fish_executor::{CommandSpec, TaskStatus};
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn task() -> Task {
        let spec = CommandSpec::new("echo").arg("work");
        Task::new("demo".to_string(), "echo work".to_string(), spec)
    }

    fn success() -> TaskOutcome {
        let mut ok = TaskOutcome::failed(&task(), String::new());
        ok.status = TaskStatus::Executed;
        ok.exit_code = Some(0);
        ok
    }

    fn preempted() -> TaskOutcome {
        TaskOutcome::failed(
            &task(),
            "fatal: worker lost (spot interruption)".to_string(),
        )
    }

    enum Script {
        PreemptThenSucceed(usize),
        AlwaysPreempt,
        GenuineFailure,
    }

    struct ScriptedPrimary {
        script: Script,
        calls: AtomicUsize,
    }

    impl ScriptedPrimary {
        fn new(script: Script) -> Self {
            Self {
                script,
                calls: AtomicUsize::new(0),
            }
        }
    }

    impl TaskExecutor for ScriptedPrimary {
        fn execute(&self, _task: &Task) -> Result<TaskOutcome, ExecutorError> {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            match &self.script {
                Script::AlwaysPreempt => Ok(preempted()),
                Script::PreemptThenSucceed(n) if call < *n => Ok(preempted()),
                Script::GenuineFailure => Ok(TaskOutcome::failed(
                    &task(),
                    "error[E0432]: unresolved import".to_string(),
                )),
                _ => Ok(success()),
            }
        }
    }

    struct FallbackProbe {
        calls: AtomicUsize,
    }

    impl TaskExecutor for FallbackProbe {
        fn execute(&self, _task: &Task) -> Result<TaskOutcome, ExecutorError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(success())
        }
    }

    #[test]
    fn test_preempted_task_retries_then_succeeds_on_primary() {
        let primary = Arc::new(ScriptedPrimary::new(Script::PreemptThenSucceed(2)));
        let fallback = Arc::new(FallbackProbe {
            calls: AtomicUsize::new(0),
        });
        let exec = PreemptionRetryExecutor::new(
            Arc::clone(&primary),
            Arc::clone(&fallback),
            3,
            Duration::ZERO,
        );

        let outcome = exec.execute(&task()).unwrap();
        assert_eq!(outcome.status, TaskStatus::Executed);
        // one initial attempt plus two preemption retries
        assert_eq!(primary.calls.load(Ordering::SeqCst), 3);
        assert_eq!(fallback.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_genuine_failure_is_returned_immediately() {
        let primary = Arc::new(ScriptedPrimary::new(Script::GenuineFailure));
        let fallback = Arc::new(FallbackProbe {
            calls: AtomicUsize::new(0),
        });
        let exec = PreemptionRetryExecutor::new(
            Arc::clone(&primary),
            Arc::clone(&fallback),
            3,
            Duration::ZERO,
        );

        let outcome = exec.execute(&task()).unwrap();
        assert_eq!(outcome.status, TaskStatus::Failed);
        assert!(outcome.stderr.contains("unresolved import"));
        assert_eq!(
            primary.calls.load(Ordering::SeqCst),
            1,
            "compile errors must never retry"
        );
        assert_eq!(fallback.calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_exhausted_primary_migrates_to_fallback() {
        let primary = Arc::new(ScriptedPrimary::new(Script::AlwaysPreempt));
        let fallback = Arc::new(FallbackProbe {
            calls: AtomicUsize::new(0),
        });
        let exec = PreemptionRetryExecutor::new(
            Arc::clone(&primary),
            Arc::clone(&fallback),
            3,
            Duration::ZERO,
        );

        let outcome = exec.execute(&task()).unwrap();
        assert_eq!(outcome.status, TaskStatus::Executed);
        assert_eq!(primary.calls.load(Ordering::SeqCst), 3);
        assert_eq!(fallback.calls.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_marker_classification_is_case_insensitive() {
        assert!(is_preemption_failure("Node PREEMPTED by provider"));
        assert!(is_preemption_failure("connection reset by peer"));
        assert!(!is_preemption_failure("error: linking failed"));
    }
}
