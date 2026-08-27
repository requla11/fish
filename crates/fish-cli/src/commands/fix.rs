use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, ExitCode, Stdio};

use crate::args::FixArgs;

#[derive(Debug, Clone, PartialEq)]
pub struct CodeEdit {
    pub file: PathBuf,
    pub byte_start: usize,
    pub byte_end: usize,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FixSuggestion {
    pub title: String,
    pub error_code: Option<String>,
    pub file: PathBuf,
    pub line: usize,
    pub col: usize,
    pub edits: Vec<CodeEdit>,
    pub diff: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Diagnostic {
    pub file: Option<String>,
    pub line: Option<u64>,
    pub col: Option<u64>,
    pub message: String,
    pub code: Option<String>,
    pub suggestions: Vec<CodeEdit>,
}

pub fn parse_cargo_diagnostics(stdout: &str, base_dir: &Path) -> Vec<Diagnostic> {
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
        if !level.starts_with("error") && !level.starts_with("warning") {
            continue;
        }

        let text = message
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("")
            .to_string();

        let code = message
            .get("code")
            .and_then(|c| c.get("code"))
            .and_then(|code_str| code_str.as_str())
            .map(str::to_string);

        let mut primary_file = None;
        let mut primary_line = None;
        let mut primary_col = None;
        let mut suggestions = Vec::new();

        if let Some(spans) = message.get("spans").and_then(|s| s.as_array()) {
            for span in spans {
                let is_primary = span
                    .get("is_primary")
                    .and_then(|p| p.as_bool())
                    .unwrap_or(false);
                let file_name = span
                    .get("file_name")
                    .and_then(|f| f.as_str())
                    .map(str::to_string);
                let line_start = span.get("line_start").and_then(|l| l.as_u64());
                let col_start = span.get("column_start").and_then(|c| c.as_u64());

                if is_primary || primary_file.is_none() {
                    primary_file = file_name.clone();
                    primary_line = line_start;
                    primary_col = col_start;
                }

                let replacement = span.get("suggested_replacement").and_then(|r| r.as_str());
                let b_start = span.get("byte_start").and_then(|b| b.as_u64());
                let b_end = span.get("byte_end").and_then(|b| b.as_u64());

                if let (Some(rep), Some(f), Some(start), Some(end)) =
                    (replacement, file_name.as_ref(), b_start, b_end)
                {
                    suggestions.push(CodeEdit {
                        file: base_dir.join(f),
                        byte_start: start as usize,
                        byte_end: end as usize,
                        replacement: rep.to_string(),
                    });
                }
            }
        }

        if let Some(children) = message.get("children").and_then(|c| c.as_array()) {
            for child in children {
                if let Some(spans) = child.get("spans").and_then(|s| s.as_array()) {
                    for span in spans {
                        let replacement =
                            span.get("suggested_replacement").and_then(|r| r.as_str());
                        let file_name = span
                            .get("file_name")
                            .and_then(|f| f.as_str())
                            .map(str::to_string)
                            .or_else(|| primary_file.clone());
                        let b_start = span.get("byte_start").and_then(|b| b.as_u64());
                        let b_end = span.get("byte_end").and_then(|b| b.as_u64());

                        if let (Some(rep), Some(f), Some(start), Some(end)) =
                            (replacement, file_name.as_ref(), b_start, b_end)
                        {
                            suggestions.push(CodeEdit {
                                file: base_dir.join(f),
                                byte_start: start as usize,
                                byte_end: end as usize,
                                replacement: rep.to_string(),
                            });
                        }
                    }
                }
            }
        }

        diagnostics.push(Diagnostic {
            file: primary_file,
            line: primary_line,
            col: primary_col,
            message: text,
            code,
            suggestions,
        });
    }

    diagnostics
}

pub fn apply_edits_to_string(source: &str, edits: &[CodeEdit]) -> Result<String, String> {
    let mut sorted_edits = edits.to_vec();
    sorted_edits.sort_by_key(|a| std::cmp::Reverse(a.byte_start));

    let mut result = source.to_string();
    for edit in sorted_edits {
        if edit.byte_start > result.len() || edit.byte_end > result.len() {
            return Err("edit byte offsets exceed file length".to_string());
        }
        if edit.byte_start > edit.byte_end {
            return Err("invalid edit: byte_start exceeds byte_end".to_string());
        }
        result.replace_range(edit.byte_start..edit.byte_end, &edit.replacement);
    }
    Ok(result)
}

pub fn generate_unified_diff(file_path: &Path, original: &str, modified: &str) -> String {
    if original == modified {
        return String::new();
    }

    let orig_lines: Vec<&str> = original.lines().collect();
    let mod_lines: Vec<&str> = modified.lines().collect();

    let mut diff = String::new();
    diff.push_str(&format!("--- a/{}\n", file_path.display()));
    diff.push_str(&format!("+++ b/{}\n", file_path.display()));

    let mut i = 0;
    let mut j = 0;

    while i < orig_lines.len() || j < mod_lines.len() {
        if i < orig_lines.len() && j < mod_lines.len() && orig_lines[i] == mod_lines[j] {
            i += 1;
            j += 1;
        } else {
            let start_i = i;
            let start_j = j;

            while i < orig_lines.len() && (j >= mod_lines.len() || orig_lines[i] != mod_lines[j]) {
                i += 1;
            }
            while j < mod_lines.len() && (i >= orig_lines.len() || orig_lines[i] != mod_lines[j]) {
                j += 1;
            }

            let orig_count = i - start_i;
            let mod_count = j - start_j;

            diff.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                start_i + 1,
                orig_count.max(1),
                start_j + 1,
                mod_count.max(1)
            ));

            for line in &orig_lines[start_i..i] {
                diff.push_str(&format!("-{line}\n"));
            }
            for line in &mod_lines[start_j..j] {
                diff.push_str(&format!("+{line}\n"));
            }
        }
    }

    diff
}

pub fn extract_fix_proposals(diagnostics: &[Diagnostic], base_dir: &Path) -> Vec<FixSuggestion> {
    let mut proposals = Vec::new();

    for diag in diagnostics {
        let Some(file_str) = diag.file.as_deref() else {
            continue;
        };
        let file_path = base_dir.join(file_str);
        let Ok(original_content) = std::fs::read_to_string(&file_path) else {
            continue;
        };

        let mut edits = diag.suggestions.clone();
        if edits.is_empty() {
            edits.extend(infer_grounded_edit(diag, &file_path, &original_content));
        }

        if edits.is_empty() {
            continue;
        }

        if let Ok(modified_content) = apply_edits_to_string(&original_content, &edits) {
            let diff =
                generate_unified_diff(Path::new(file_str), &original_content, &modified_content);
            if !diff.is_empty() {
                proposals.push(FixSuggestion {
                    title: diag.message.clone(),
                    error_code: diag.code.clone(),
                    file: file_path,
                    line: diag.line.unwrap_or(1) as usize,
                    col: diag.col.unwrap_or(1) as usize,
                    edits,
                    diff,
                });
            }
        }
    }

    proposals
}

fn infer_grounded_edit(diag: &Diagnostic, file_path: &Path, content: &str) -> Option<CodeEdit> {
    let line_no = diag.line? as usize;
    let lines: Vec<&str> = content.lines().collect();
    if line_no == 0 || line_no > lines.len() {
        return None;
    }

    let line_text = lines[line_no - 1];

    let mut byte_offset = 0;
    for l in &lines[..line_no - 1] {
        byte_offset += l.len() + 1;
    }

    let msg = &diag.message;
    let is_mut_error = (msg.contains("cannot borrow") && msg.contains("as mutable"))
        || msg.contains("cannot assign twice to immutable variable")
        || diag.code.as_deref() == Some("E0384")
        || diag.code.as_deref() == Some("E0596");

    if is_mut_error {
        let let_pos = line_text.find("let ")?;
        let insert_pos = byte_offset + let_pos + 4;
        if !line_text[let_pos..].starts_with("let mut ") {
            return Some(CodeEdit {
                file: file_path.to_path_buf(),
                byte_start: insert_pos,
                byte_end: insert_pos,
                replacement: "mut ".to_string(),
            });
        }
    }

    let is_unused_var =
        msg.contains("unused variable:") || diag.code.as_deref() == Some("unused_variables");
    if is_unused_var {
        let var_name = msg.split('`').nth(1)?;
        let var_pos = line_text.find(var_name)?;
        let target_pos = byte_offset + var_pos;
        return Some(CodeEdit {
            file: file_path.to_path_buf(),
            byte_start: target_pos,
            byte_end: target_pos,
            replacement: "_".to_string(),
        });
    }

    if msg.contains("expected `;`") || msg.contains("expected `;`, found") {
        let line_end_pos = byte_offset + line_text.trim_end().len();
        return Some(CodeEdit {
            file: file_path.to_path_buf(),
            byte_start: line_end_pos,
            byte_end: line_end_pos,
            replacement: ";".to_string(),
        });
    }

    None
}

pub fn apply_suggestions(suggestions: &[FixSuggestion]) -> Result<usize, String> {
    let mut file_edits: HashMap<PathBuf, Vec<CodeEdit>> = HashMap::new();

    for sug in suggestions {
        for edit in &sug.edits {
            file_edits
                .entry(edit.file.clone())
                .or_default()
                .push(edit.clone());
        }
    }

    let count = file_edits.len();
    for (path, edits) in file_edits {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let modified = apply_edits_to_string(&content, &edits)?;
        std::fs::write(&path, modified)
            .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    }

    Ok(count)
}

fn collect_cargo_diagnostics(project_dir: &Path) -> Result<Vec<Diagnostic>, String> {
    let output = ProcessCommand::new("cargo")
        .args(["check", "--message-format=json", "--quiet"])
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("failed to spawn cargo: {e}"))?;
    Ok(parse_cargo_diagnostics(
        &String::from_utf8_lossy(&output.stdout),
        project_dir,
    ))
}

pub fn run_fix(args: FixArgs) -> ExitCode {
    let target_path = args.path.unwrap_or_else(|| PathBuf::from("."));
    println!("=== Fish Compiler-Grounded Fix Engine ===");
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

    let proposals = extract_fix_proposals(&diagnostics, &target_path);

    if args.diff {
        if proposals.is_empty() {
            println!("No automatic diffs available for current diagnostics.");
        } else {
            for (idx, prop) in proposals.iter().enumerate() {
                let code_label = prop
                    .error_code
                    .as_deref()
                    .map(|c| format!(" ({c})"))
                    .unwrap_or_default();
                println!("# Fix #{}: {}{}", idx + 1, prop.title, code_label);
                print!("{}", prop.diff);
            }
        }
        return ExitCode::SUCCESS;
    }

    println!("\nDiagnostics Summary ({} item(s)):", diagnostics.len());
    for diag in &diagnostics {
        let code_label = diag
            .code
            .as_deref()
            .map(|c| format!(" [{c}]"))
            .unwrap_or_default();
        match (&diag.file, diag.line) {
            (Some(file), Some(line)) => println!("  • {file}:{line}:{code_label} {}", diag.message),
            (Some(file), None) => println!("  • {file}:{code_label} {}", diag.message),
            _ => println!("  •{code_label} {}", diag.message),
        }
    }

    if !proposals.is_empty() {
        println!(
            "\nCompiler-Grounded Fix Proposals ({} available):",
            proposals.len()
        );
        for (idx, prop) in proposals.iter().enumerate() {
            let code_label = prop
                .error_code
                .as_deref()
                .map(|c| format!(" ({c})"))
                .unwrap_or_default();
            println!("\n[{}] {}{}", idx + 1, prop.title, code_label);
            print!("{}", prop.diff);
        }
    }

    if args.apply {
        if !proposals.is_empty() {
            println!(
                "\nApplying {} compiler-grounded fix proposal(s)…",
                proposals.len()
            );
            match apply_suggestions(&proposals) {
                Ok(files) => {
                    println!("Successfully applied edits to {files} file(s).");
                }
                Err(err) => {
                    eprintln!("error applying grounded edits: {err}");
                }
            }
        }

        println!(
            "Running `cargo fix --allow-dirty --allow-staged` for complementary toolchain fixes…"
        );
        match crate::self_heal::attempt_cargo_auto_fix(&target_path) {
            Some(Ok(output)) => {
                if output.trim().is_empty() {
                    println!("cargo fix completed.");
                } else {
                    for line in output.lines().take(20) {
                        println!("  {line}");
                    }
                }
                println!("Applied. Re-run `fish fix` to verify zero diagnostics remain.");
            }
            Some(Err(err)) => {
                eprintln!("error: cargo fix failed to spawn: {err}");
            }
            None => {
                eprintln!("error: no Cargo.toml found; --apply requires a Cargo project.");
            }
        }
    } else if !proposals.is_empty() {
        println!("\nRun `fish fix --apply` to automatically apply these suggested edits.");
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
                println!("\nAI Remediation Proposal:");
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
    use tempfile::tempdir;

    #[test]
    fn test_parse_cargo_diagnostics_extracts_errors_with_spans() {
        let stdout = concat!(
            r#"{"reason":"compiler-artifact","target":{"name":"x"}}"#,
            "\n",
            r#"{"reason":"compiler-message","message":{"level":"error","message":"mismatched types","code":{"code":"E0308"},"spans":[{"file_name":"src/main.rs","line_start":12,"line_end":12,"column_start":5,"suggested_replacement":"&val","byte_start":100,"byte_end":103,"is_primary":true}]}}"#,
            "\n",
            r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused","spans":[]}}"#,
            "\n",
            "not json at all",
            "\n",
            r#"{"reason":"compiler-message","message":{"level":"error","message":"cannot find value","spans":[]}}"#,
            "\n"
        );

        let diags = parse_cargo_diagnostics(stdout, Path::new("/workspace"));
        assert_eq!(diags.len(), 3);
        assert_eq!(diags[0].message, "mismatched types");
        assert_eq!(diags[0].file.as_deref(), Some("src/main.rs"));
        assert_eq!(diags[0].line, Some(12));
        assert_eq!(diags[0].code.as_deref(), Some("E0308"));
        assert_eq!(diags[0].suggestions.len(), 1);
        assert_eq!(diags[0].suggestions[0].replacement, "&val");
        assert_eq!(diags[2].message, "cannot find value");
        assert_eq!(diags[2].file, None);
    }

    #[test]
    fn test_parse_cargo_diagnostics_child_suggestions() {
        let stdout = r#"{"reason":"compiler-message","message":{"level":"error","message":"cannot borrow as mutable","code":{"code":"E0596"},"spans":[{"file_name":"src/lib.rs","line_start":5,"is_primary":true}],"children":[{"level":"help","message":"consider changing this to be mutable","spans":[{"file_name":"src/lib.rs","byte_start":20,"byte_end":20,"suggested_replacement":"mut "}]}]}}"#;
        let diags = parse_cargo_diagnostics(stdout, Path::new("/workspace"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].suggestions.len(), 1);
        assert_eq!(diags[0].suggestions[0].replacement, "mut ");
        assert_eq!(diags[0].suggestions[0].byte_start, 20);
    }

    #[test]
    fn test_apply_edits_to_string() {
        let source = "let x = 5;\nlet y = 10;";
        let edits = vec![
            CodeEdit {
                file: PathBuf::from("src/main.rs"),
                byte_start: 4,
                byte_end: 4,
                replacement: "mut ".to_string(),
            },
            CodeEdit {
                file: PathBuf::from("src/main.rs"),
                byte_start: 15,
                byte_end: 15,
                replacement: "mut ".to_string(),
            },
        ];

        let res = apply_edits_to_string(source, &edits).unwrap();
        assert_eq!(res, "let mut x = 5;\nlet mut y = 10;");
    }

    #[test]
    fn test_generate_unified_diff() {
        let orig = "fn main() {\n    let x = 5;\n}\n";
        let modif = "fn main() {\n    let mut x = 5;\n}\n";
        let diff = generate_unified_diff(Path::new("src/main.rs"), orig, modif);
        assert!(diff.contains("--- a/src/main.rs"));
        assert!(diff.contains("+++ b/src/main.rs"));
        assert!(diff.contains("-    let x = 5;"));
        assert!(diff.contains("+    let mut x = 5;"));
    }

    #[test]
    fn test_extract_and_apply_proposals() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        let main_file = src_dir.join("main.rs");
        std::fs::write(&main_file, "fn main() {\n    let count = 10;\n}\n").unwrap();

        let diag = Diagnostic {
            file: Some("src/main.rs".to_string()),
            line: Some(2),
            col: Some(9),
            message: "unused variable: `count`".to_string(),
            code: Some("unused_variables".to_string()),
            suggestions: Vec::new(),
        };

        let proposals = extract_fix_proposals(std::slice::from_ref(&diag), dir.path());
        assert_eq!(proposals.len(), 1);
        assert!(proposals[0].diff.contains("let _count = 10;"));

        let count = apply_suggestions(&proposals).unwrap();
        assert_eq!(count, 1);

        let updated_content = std::fs::read_to_string(&main_file).unwrap();
        assert!(updated_content.contains("let _count = 10;"));
    }

    #[test]
    fn test_infer_missing_mut_edit() {
        let dir = tempdir().unwrap();
        let src_file = dir.path().join("lib.rs");
        std::fs::write(
            &src_file,
            "fn compute() {\n    let val = 1;\n    val = 2;\n}\n",
        )
        .unwrap();

        let diag = Diagnostic {
            file: Some("lib.rs".to_string()),
            line: Some(2),
            col: Some(9),
            message: "cannot assign twice to immutable variable `val`".to_string(),
            code: Some("E0384".to_string()),
            suggestions: Vec::new(),
        };

        let proposals = extract_fix_proposals(std::slice::from_ref(&diag), dir.path());
        assert_eq!(proposals.len(), 1);
        assert!(proposals[0].diff.contains("let mut val = 1;"));
    }
}
