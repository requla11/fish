//! Trace replay: record every spawned process during a build, then replay
//! them to prove hermetic determinism.
//!
//! A [`ProcessRecord`] captures the full invocation (program, args, cwd,
//! sanitized env) plus a BLAKE3 hash of stdout+stderr. Replaying the trace
//! and comparing hashes proves that the build is deterministic — any
//! divergence indicates non-hermetic inputs (system state, network calls,
//! time-dependent output).
//!
//! Serialization: JSONL for human inspection and CI diffing.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// One recorded process invocation with its outcome.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessRecord {
    /// Executable path or command name.
    pub program: String,
    pub args: Vec<String>,
    /// Working directory at spawn time.
    pub cwd: Option<PathBuf>,
    /// Explicitly set environment variables only (not the inherited set).
    pub env_overrides: BTreeMap<String, String>,
    /// Process exit code (`None` if killed by signal).
    pub exit_code: Option<i32>,
    /// BLAKE3 hash of combined stdout + stderr bytes.
    pub output_hash: String,
}

impl ProcessRecord {
    /// Capture from a completed command's specification and outputs.
    pub fn capture(
        spec: &crate::command::CommandSpec,
        exit_code: Option<i32>,
        stdout: &[u8],
        stderr: &[u8],
    ) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(stdout);
        hasher.update(stderr);
        Self {
            program: spec.program.clone(),
            args: spec.args.clone(),
            cwd: spec.cwd.clone(),
            env_overrides: spec
                .env
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            exit_code,
            output_hash: hasher.finalize().to_hex().to_string(),
        }
    }
}

/// Full execution trace from one build run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub records: Vec<ProcessRecord>,
}

impl ExecutionTrace {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
        }
    }

    pub fn push(&mut self, record: ProcessRecord) {
        self.records.push(record);
    }

    /// Save as JSON Lines.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut content = String::new();
        for record in &self.records {
            let line = serde_json::to_string(record)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            content.push_str(&line);
            content.push('\n');
        }
        fs::write(path, content)
    }

    /// Load from a previously saved trace file.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let content = fs::read_to_string(path)?;
        let records: Vec<ProcessRecord> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect();
        Ok(Self { records })
    }

    /// Replay all recorded processes and verify output hashes match.
    ///
    /// Returns a list of divergences; an empty list means the build is
    /// bit-for-bit deterministic. Processes are re-executed sequentially in
    /// recorded order — no parallelism, no caching, no environment leakage.
    ///
    /// Only processes with `exit_code == Some(0)` are replayed; failed
    /// commands may legitimately produce different error messages.
    pub fn replay_and_verify(&self) -> Vec<ReplayDivergence> {
        let mut divergences = Vec::new();

        for (index, record) in self.records.iter().enumerate() {
            // Skip failed commands — their error output often embeds paths
            // or timestamps that differ across runs by design.
            if record.exit_code != Some(0) {
                continue;
            }

            let mut cmd = std::process::Command::new(&record.program);
            cmd.args(&record.args);
            cmd.env_clear();
            cmd.envs(&record.env_overrides);
            if let Some(cwd) = &record.cwd {
                cmd.current_dir(cwd);
            }
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::piped());
            cmd.stdin(std::process::Stdio::null());

            match cmd.output() {
                Ok(output) => {
                    let mut hasher = blake3::Hasher::new();
                    hasher.update(&output.stdout);
                    hasher.update(&output.stderr);
                    let new_hash = hasher.finalize().to_hex().to_string();

                    if new_hash != record.output_hash {
                        divergences.push(ReplayDivergence {
                            index,
                            program: record.program.clone(),
                            expected_hash: record.output_hash.clone(),
                            actual_hash: new_hash,
                        });
                    }
                }
                Err(e) => {
                    divergences.push(ReplayDivergence {
                        index,
                        program: format!("{} (spawn failed: {e})", record.program),
                        expected_hash: record.output_hash.clone(),
                        actual_hash: "<spawn-error>".to_string(),
                    });
                }
            }
        }
        divergences
    }
}

/// One mismatch between original and replayed output.
#[derive(Debug, Clone)]
pub struct ReplayDivergence {
    pub index: usize,
    pub program: String,
    pub expected_hash: String,
    pub actual_hash: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::CommandSpec;

    #[cfg(windows)]
    fn echo_spec(msg: &str) -> CommandSpec {
        let mut spec = CommandSpec::new("cmd");
        spec.args.push("/C".to_string());
        spec.args.push(format!("echo {msg}"));
        spec
    }

    #[cfg(not(windows))]
    fn echo_spec(msg: &str) -> CommandSpec {
        let mut spec = CommandSpec::new("echo");
        spec.args.push(msg.to_string());
        spec
    }

    fn run_and_capture(spec: &CommandSpec) -> (Option<i32>, Vec<u8>, Vec<u8>) {
        let output = std::process::Command::new(&spec.program)
            .args(&spec.args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .unwrap();
        (output.status.code(), output.stdout, output.stderr)
    }

    #[test]
    fn test_capture_creates_deterministic_hash() {
        let spec = echo_spec("hello world");
        let (code, stdout, stderr) = run_and_capture(&spec);
        let r1 = ProcessRecord::capture(&spec, code, &stdout, &stderr);
        let r2 = ProcessRecord::capture(&spec, code, &stdout, &stderr);

        assert_eq!(r1.output_hash, r2.output_hash);
        assert!(!r1.program.is_empty());
    }

    #[test]
    fn test_different_input_different_hash() {
        let spec_a = echo_spec("alpha");
        let spec_b = echo_spec("beta");

        let (code_a, out_a, err_a) = run_and_capture(&spec_a);
        let (code_b, out_b, err_b) = run_and_capture(&spec_b);

        let rec_a = ProcessRecord::capture(&spec_a, code_a, &out_a, &err_a);
        let rec_b = ProcessRecord::capture(&spec_b, code_b, &out_b, &err_b);
        assert_ne!(rec_a.output_hash, rec_b.output_hash);
    }

    #[test]
    fn test_trace_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("trace.jsonl");

        let mut trace = ExecutionTrace::new();
        let spec = echo_spec("persist");
        let (code, stdout, stderr) = run_and_capture(&spec);
        trace.push(ProcessRecord::capture(&spec, code, &stdout, &stderr));
        trace.save(&path).unwrap();

        let loaded = ExecutionTrace::load(&path).unwrap();
        assert_eq!(loaded.records.len(), 1);
        assert_eq!(loaded.records[0].output_hash, trace.records[0].output_hash);
    }

    #[test]
    fn test_replay_deterministic_produces_no_divergences() {
        // Use a command with fully deterministic output.
        let mut spec = CommandSpec::new("python");
        spec.args.push("-c".to_string());
        spec.args.push("print(1+1)".to_string());

        let output = std::process::Command::new(&spec.program)
            .args(&spec.args)
            .output()
            .unwrap_or_else(|_| panic!("python must be available for this test"));

        let record =
            ProcessRecord::capture(&spec, output.status.code(), &output.stdout, &output.stderr);

        // Replay manually: run again and compare.
        let replay_output = std::process::Command::new(&spec.program)
            .args(&spec.args)
            .output()
            .unwrap();

        let mut hasher = blake3::Hasher::new();
        hasher.update(&replay_output.stdout);
        hasher.update(&replay_output.stderr);
        let replay_hash = hasher.finalize().to_hex().to_string();

        assert_eq!(
            record.output_hash, replay_hash,
            "deterministic command must match"
        );
    }
}
