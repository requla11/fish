//! Integration tests for the offline `apple` shim.
//!
//! These pin down the fallback sandbox behaviour `fish-sandbox` relies on
//! while the real `submodules/apple` daemon is unavailable: request/result
//! protocol semantics, execution and output capture, and the determinism
//! verifier stub.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use apple::protocol::{ExecutionRequest, ExecutionResult, IsolationLevel, SandboxProfile};
use apple::{AppleDaemonServer, DeterminismVerifier};

// `tempfile::TempDir` deletes on drop, so keep it alive alongside the path.
struct TempGuard {
    _dir: tempfile::TempDir,
    path: PathBuf,
}

// This helper is not itself a `#[test]`, so the workspace-level
// `clippy::unwrap_used` needs a targeted opt-out.
#[allow(clippy::unwrap_used)]
fn temp_guard() -> TempGuard {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().to_path_buf();
    TempGuard { _dir: dir, path }
}

fn request(task_id: &str, working_dir: &Path, argv: &[&str]) -> ExecutionRequest {
    ExecutionRequest {
        task_id: task_id.to_string(),
        working_dir: working_dir.to_path_buf(),
        argv: argv.iter().map(|s| (*s).to_string()).collect(),
        env: HashMap::new(),
        profile: SandboxProfile::default(),
        keep_jail: false,
    }
}

// ---------------------------------------------------------------------------
// protocol
// ---------------------------------------------------------------------------

#[test]
fn isolation_level_defaults_to_strict_filesystem() {
    assert_eq!(IsolationLevel::default(), IsolationLevel::StrictFilesystem);
}

#[test]
fn sandbox_profile_deserializes_with_defaults() {
    // Only `level` is required; outputs must default to empty.
    let json = r#"{"level": "FullHermetic", "declared_inputs": ["a.txt"]}"#;
    let profile: SandboxProfile = serde_json::from_str(json).unwrap();
    assert_eq!(profile.level, IsolationLevel::FullHermetic);
    assert_eq!(profile.declared_inputs, vec![PathBuf::from("a.txt")]);
    assert!(profile.declared_outputs.is_empty());
}

#[test]
fn execution_result_deserializes_with_hermetic_default() {
    let json = r#"{"task_id": "t1", "exit_code": 0}"#;
    let result: ExecutionResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.task_id, "t1");
    assert_eq!(result.exit_code, 0);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    assert!(result.hermetic_guarantee, "missing flag defaults to true");
}

#[test]
fn execution_request_round_trips_through_json() {
    let req = request("task-7", Path::new("/tmp"), &["echo", "hi"]);
    let json = serde_json::to_string(&req).unwrap();
    let back: ExecutionRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(back.task_id, "task-7");
    assert_eq!(back.argv, vec!["echo", "hi"]);
    assert_eq!(back.profile.level, IsolationLevel::StrictFilesystem);
}

// ---------------------------------------------------------------------------
// AppleDaemonServer
// ---------------------------------------------------------------------------

#[tokio::test]
async fn executes_command_and_captures_stdout() {
    let scratch = temp_guard();
    let server = AppleDaemonServer::new(scratch.path.join("scratch"));
    let result = server
        .execute_task(request("t-echo", &scratch.path, &["echo", "hello"]))
        .await;

    assert_eq!(result.task_id, "t-echo");
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stdout, "hello\n");
    assert!(result.stderr.is_empty());
    assert!(result.hermetic_guarantee);
}

#[tokio::test]
async fn executes_command_with_custom_environment() {
    let scratch = temp_guard();
    let server = AppleDaemonServer::new(scratch.path.join("scratch"));
    let mut req = request(
        "t-env",
        &scratch.path,
        &["/bin/sh", "-c", "echo $FISH_TEST_VAR"],
    );
    req.env
        .insert("FISH_TEST_VAR".to_string(), "injected".to_string());

    let result = server.execute_task(req).await;
    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert_eq!(result.stdout, "injected\n");
}

#[tokio::test]
async fn runs_command_in_requested_working_dir() {
    let scratch = temp_guard();
    let workdir = scratch.path.join("workdir");
    std::fs::create_dir_all(&workdir).unwrap();

    let server = AppleDaemonServer::new(scratch.path.join("scratch"));
    let req = request("t-cwd", &workdir, &["/bin/sh", "-c", "pwd"]);
    let result = server.execute_task(req).await;

    assert_eq!(result.exit_code, 0, "stderr: {}", result.stderr);
    assert_eq!(PathBuf::from(result.stdout.trim()), workdir);
}

#[tokio::test]
async fn reports_nonzero_exit_and_failed_hermetic_guarantee() {
    let scratch = temp_guard();
    let server = AppleDaemonServer::new(scratch.path.join("scratch"));
    let result = server
        .execute_task(request(
            "t-fail",
            &scratch.path,
            &["/bin/sh", "-c", "exit 3"],
        ))
        .await;

    assert_eq!(result.exit_code, 3);
    assert!(!result.hermetic_guarantee);
}

#[tokio::test]
async fn empty_argv_returns_exit_127() {
    let scratch = temp_guard();
    let server = AppleDaemonServer::new(scratch.path.join("scratch"));
    let result = server
        .execute_task(request("t-empty", &scratch.path, &[]))
        .await;

    assert_eq!(result.task_id, "t-empty");
    assert_eq!(result.exit_code, 127);
    assert!(
        result.stderr.contains("empty argv"),
        "stderr: {}",
        result.stderr
    );
    assert!(!result.hermetic_guarantee);
}

#[tokio::test]
async fn missing_program_returns_spawn_error() {
    let scratch = temp_guard();
    let server = AppleDaemonServer::new(scratch.path.join("scratch"));
    let result = server
        .execute_task(request(
            "t-missing",
            &scratch.path,
            &["definitely-not-a-real-binary-xyz"],
        ))
        .await;

    assert_eq!(result.exit_code, 127);
    assert!(
        result.stderr.contains("failed to spawn"),
        "stderr: {}",
        result.stderr
    );
    assert!(!result.hermetic_guarantee);
}

// ---------------------------------------------------------------------------
// DeterminismVerifier
// ---------------------------------------------------------------------------

#[tokio::test]
async fn verifier_reports_reproducible_for_successful_task() {
    let scratch = temp_guard();
    let verifier = DeterminismVerifier::new(scratch.path.join("scratch"));
    let report = verifier
        .verify_reproducible(
            request("t-ver", &scratch.path, &["echo", "ok"]),
            Path::new("artifact.bin"),
        )
        .await
        .unwrap();

    assert!(report.reproducible);
    assert!(
        report.details.contains("exit=0"),
        "details: {}",
        report.details
    );
}

#[tokio::test]
async fn verifier_reports_not_reproducible_for_failing_task() {
    let scratch = temp_guard();
    let verifier = DeterminismVerifier::new(scratch.path.join("scratch"));
    let report = verifier
        .verify_reproducible(
            request("t-ver-fail", &scratch.path, &["/bin/sh", "-c", "exit 1"]),
            Path::new("artifact.bin"),
        )
        .await
        .unwrap();

    assert!(!report.reproducible);
    assert!(
        report.details.contains("exit=1"),
        "details: {}",
        report.details
    );
}
