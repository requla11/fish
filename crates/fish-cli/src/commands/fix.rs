use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode, Stdio};

use crate::args::FixArgs;

#[derive(Debug, Clone, PartialEq)]
struct Diagnostic {
    file: Option<String>,
    line: Option<u64>,
    message: String,
}

/// Parse `cargo check --message-format=json` output into error diagnostics.
fn parse_cargo_diagnostics(stdout: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("reason").and_then(|r| r.as_str()) != Some("compiler-message") {
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        let level = message.get("level").and_then(|l| l.as_str()).unwrap_or("");
        if !level.starts_with("error") {
            continue;
        }
        let text = message
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();
        let (file, line_no) = message
            .get("spans")
            .and_then(|s| s.as_array())
            .and_then(|spans| spans.first())
            .map(|span| {
                (
                    span.get("file_name")
                        .and_then(|f| f.as_str())
                        .map(str::to_string),
                    span.get("line_start").and_then(|l| l.as_u64()),
                )
            })
            .unwrap_or((None, None));
        diagnostics.push(Diagnostic {
            file,
            line: line_no,
            message: text,
        });
    }
    diagnostics
}

fn collect_cargo_diagnostics(project_dir: &Path) -> Result<Vec<Diagnostic>, String> {
    let output = ProcessCommand::new("cargo")
        .args(["check", "--message-format=json", "--quiet"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("failed to spawn cargo: {e}"))?;
    Ok(parse_cargo_diagnostics(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

pub fn run_fix(args: FixArgs) -> ExitCode {
    let target_path = args.path.unwrap_or_else(|| PathBuf::from("."));
    println!("=== Fish Auto-Healer & AI Diagnostics ===");
    println!("Scanning project at: {}", target_path.display());

    if !target_path.join("Cargo.toml").exists() {
        println!("No Cargo.toml found; fish fix currently diagnoses Cargo projects only.");
        return ExitCode::SUCCESS;
    }

    let diagnostics = match collect_cargo_diagnostics(&target_path) {
        Ok(diags) => diags,
        Err(err) => {
            eprintln!("error: unable to run cargo check: {err}");
            return ExitCode::FAILURE;
        }
    };

    if diagnostics.is_empty() {
        println!("Project Status: Clean — `cargo check` reported no errors.");
        return ExitCode::SUCCESS;
    }

    println!("Diagnostics Summary:");
    for diag in &diagnostics {
        match (&diag.file, diag.line) {
            (Some(file), Some(line)) => println!("  • {file}:{line}: {}", diag.message),
            (Some(file), None) => println!("  • {file}: {}", diag.message),
            _ => println!("  • {}", diag.message),
        }
    }

    if args.apply {
        println!("Applying `cargo fix --allow-dirty --allow-staged`…");
        match crate::self_heal::attempt_cargo_auto_fix(&target_path) {
            Some(Ok(output)) => {
                if output.trim().is_empty() {
                    println!("cargo fix completed with no output (nothing to change).");
                } else {
                    for line in output.lines().take(20) {
                        println!("  {line}");
                    }
                }
                println!("Applied. Re-run `fish fix` to confirm zero diagnostics remain.");
            }
            Some(Err(err)) => {
                eprintln!("error: cargo fix failed to spawn: {err}");
            }
            None => {
                eprintln!("error: no Cargo.toml found; --apply requires a Cargo project.");
            }
        }
    }

    if args.ai {
        let primary = &diagnostics[0];
        let file_content = primary
            .file
            .as_ref()
            .map(|file| target_path.join(file))
            .and_then(|path| std::fs::read_to_string(path).ok());
        let Some(file_content) = file_content else {
            println!("--ai skipped: the diagnostic does not reference a readable source file.");
            return ExitCode::SUCCESS;
        };
        match query_ai_autofix(&file_content, &primary.message) {
            Ok(Some(proposal)) => {
                println!("AI Remediation Proposal:");
                println!(
                    "  Status: {}",
                    proposal
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("(no status)")
                );
                println!(
                    "  Explanation: {}",
                    proposal
                        .get("explanation")
                        .and_then(|v| v.as_str())
                        .unwrap_or("(no explanation)")
                );
                if let Some(modified) = proposal.get("modified").and_then(|v| v.as_bool()) {
                    println!(
                        "  Applicable fix produced: {}",
                        if modified { "yes" } else { "no" }
                    );
                }
            }
            Ok(None) => {
                println!("AI service unavailable; no proposal could be fetched.");
            }
            Err(err) => {
                println!("AI service call failed: {err}");
            }
        }
    }

    ExitCode::SUCCESS
}

/// Send real source content plus the real compiler message to the Python AI
/// service. Returns `Ok(None)` when the service cannot be reached at all.
fn query_ai_autofix(
    file_content: &str,
    error_message: &str,
) -> Result<Option<serde_json::Value>, String> {
    let rpc_request = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "autofix",
        "params": {
            "file_content": file_content,
            "error_message": error_message
        }
    });

    let mut child = ProcessCommand::new("python")
        .args(["-m", "fish_ai.server"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot start python -m fish_ai.server: {e}"))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = writeln!(stdin, "{rpc_request}");
    }

    let output = child
        .wait_with_output()
        .map_err(|e| format!("waiting on AI service failed: {e}"))?;
    if !output.status.success() {
        return Ok(None);
    }
    let out_str = String::from_utf8_lossy(&output.stdout);
    let Ok(resp) = serde_json::from_str::<serde_json::Value>(&out_str) else {
        return Ok(None);
    };
    Ok(resp.get("result").cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cargo_diagnostics_extracts_errors_with_spans() {
        let stdout = concat!(
            r#"{"reason":"compiler-artifact","target":{"name":"x"}}"#,
            "\n",
            r#"{"reason":"compiler-message","message":{"level":"error","message":"mismatched types","spans":[{"file_name":"src/main.rs","line_start":12,"line_end":12}]}}"#,
            "\n",
            r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused","spans":[]}}"#,
            "\n",
            "not json at all",
            "\n",
            r#"{"reason":"compiler-message","message":{"level":"error","message":"cannot find value","spans":[]}}"#,
            "\n"
        );

        let diags = parse_cargo_diagnostics(stdout);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].message, "mismatched types");
        assert_eq!(diags[0].file.as_deref(), Some("src/main.rs"));
        assert_eq!(diags[0].line, Some(12));
        assert_eq!(diags[1].message, "cannot find value");
        assert_eq!(diags[1].file, None);
    }

    #[test]
    fn test_parse_cargo_diagnostics_empty_output() {
        assert!(parse_cargo_diagnostics("").is_empty());
    }
}
