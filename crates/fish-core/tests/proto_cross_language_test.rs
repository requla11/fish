use fish_core::proto::{BuildTask, FailureAnalysisResponse, TaskResult, WorkerRegistration};
use std::collections::HashMap;

#[test]
fn test_cross_language_wire_encoding_spec() {
    let mut env = HashMap::new();
    env.insert("GOOS".to_string(), "linux".to_string());

    let task = BuildTask {
        id: "task-cross-01".to_string(),
        package_name: "polyglot".to_string(),
        toolchain: "rust".to_string(),
        command: "cargo test".to_string(),
        args: vec!["--all".to_string()],
        inputs: vec!["src/**/*.rs".to_string()],
        outputs: vec!["target/test.log".to_string()],
        dependencies: vec!["task-cross-00".to_string()],
        env,
        timeout_ms: 12000,
    };

    let encoded = task.encode();
    assert!(!encoded.is_empty());

    let decoded = BuildTask::decode(&encoded).expect("failed to decode task bytes");
    assert_eq!(task.id, decoded.id);
    assert_eq!(task.package_name, decoded.package_name);
    assert_eq!(task.toolchain, decoded.toolchain);
    assert_eq!(task.command, decoded.command);
    assert_eq!(task.args, decoded.args);
    assert_eq!(task.inputs, decoded.inputs);
    assert_eq!(task.outputs, decoded.outputs);
    assert_eq!(task.dependencies, decoded.dependencies);
    assert_eq!(task.env, decoded.env);
    assert_eq!(task.timeout_ms, decoded.timeout_ms);
}

#[test]
fn test_failure_analysis_wire_spec() {
    let resp = FailureAnalysisResponse {
        error_category: "syntax_error".to_string(),
        root_cause: "unexpected token".to_string(),
        confidence: 0.985,
        suggested_fixes: vec!["insert semicolon".to_string()],
        affected_files: vec!["lib.rs".to_string()],
    };

    let encoded = resp.encode();
    let decoded = FailureAnalysisResponse::decode(&encoded).expect("failed to decode resp bytes");
    assert_eq!(resp.error_category, decoded.error_category);
    assert_eq!(resp.root_cause, decoded.root_cause);
    assert!((resp.confidence - decoded.confidence).abs() < 1e-6);
    assert_eq!(resp.suggested_fixes, decoded.suggested_fixes);
    assert_eq!(resp.affected_files, decoded.affected_files);
}

#[test]
fn test_worker_registration_wire_spec() {
    let mut tags = HashMap::new();
    tags.insert("region".to_string(), "us-east-1".to_string());

    let reg = WorkerRegistration {
        worker_id: "node-alpha".to_string(),
        address: "10.1.2.3:50051".to_string(),
        cpu_cores: 32,
        memory_bytes: 68719476736,
        supported_toolchains: vec!["rust".to_string(), "go".to_string()],
        tags,
    };

    let encoded = reg.encode();
    let decoded = WorkerRegistration::decode(&encoded).expect("failed to decode reg bytes");
    assert_eq!(reg, decoded);
}

#[test]
fn test_task_result_wire_spec() {
    let mut digests = HashMap::new();
    digests.insert("bin".to_string(), "blake3:feedface".to_string());

    let res = TaskResult {
        task_id: "task-done".to_string(),
        exit_code: 0,
        stdout: "ok".to_string(),
        stderr: String::new(),
        duration_ms: 10,
        cached: true,
        fingerprint: "blake3:cafebabe".to_string(),
        output_digests: digests,
    };

    let encoded = res.encode();
    let decoded = TaskResult::decode(&encoded).expect("failed to decode result bytes");
    assert_eq!(res, decoded);
}
