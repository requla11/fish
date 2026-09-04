use regex::Regex;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorKind {
    MissingImport(String),
    MissingMutability(String),
    MissingClone(String),
    UnusedImport(String),
    SyntaxTypo(String),
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompilerDiagnostic {
    pub file: PathBuf,
    pub line: usize,
    pub column: usize,
    pub error_kind: ErrorKind,
    pub raw_message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemediationPatch {
    pub file: PathBuf,
    pub line: usize,
    pub old_text: String,
    pub new_text: String,
}

pub struct AutoRemediator;

impl AutoRemediator {
    pub fn parse_diagnostics(compiler_output: &str) -> Vec<CompilerDiagnostic> {
        static ERROR_REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
            Regex::new(r"(?m)^(?:error\[E\d+\]:\s+)?(.+?):(\d+):(\d+):\s+(?:error:\s+)?(.+)$")
                .unwrap()
        });
        static RUST_ALT: std::sync::LazyLock<Regex> =
            std::sync::LazyLock::new(|| Regex::new(r"-->\s+(.+?):(\d+):(\d+)").unwrap());

        let mut diagnostics = Vec::new();

        for cap in ERROR_REGEX.captures_iter(compiler_output) {
            let file_str = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
            let line: usize = cap
                .get(2)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(1);
            let col: usize = cap
                .get(3)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(1);
            let msg = cap
                .get(4)
                .map(|m| m.as_str())
                .unwrap_or_default()
                .to_string();

            let error_kind = Self::classify_error(&msg);

            diagnostics.push(CompilerDiagnostic {
                file: PathBuf::from(file_str),
                line,
                column: col,
                error_kind,
                raw_message: msg,
            });
        }

        if diagnostics.is_empty() {
            for cap in RUST_ALT.captures_iter(compiler_output) {
                let file_str = cap.get(1).map(|m| m.as_str()).unwrap_or_default();
                let line: usize = cap
                    .get(2)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1);
                let col: usize = cap
                    .get(3)
                    .and_then(|m| m.as_str().parse().ok())
                    .unwrap_or(1);

                diagnostics.push(CompilerDiagnostic {
                    file: PathBuf::from(file_str),
                    line,
                    column: col,
                    error_kind: ErrorKind::Unknown(compiler_output.to_string()),
                    raw_message: compiler_output.to_string(),
                });
            }
        }

        diagnostics
    }

    fn classify_error(msg: &str) -> ErrorKind {
        if msg.contains("cannot find type")
            || msg.contains("not found in this scope")
            || msg.contains("undefined name")
        {
            ErrorKind::MissingImport(msg.to_string())
        } else if msg.contains("cannot borrow")
            || msg.contains("as mutable")
            || msg.contains("needs to be mutable")
            || msg.contains("immutable variable")
            || msg.contains("cannot assign twice")
        {
            ErrorKind::MissingMutability(msg.to_string())
        } else if msg.contains("use of moved value") || msg.contains("move occurs because") {
            ErrorKind::MissingClone(msg.to_string())
        } else if msg.contains("unused import") {
            ErrorKind::UnusedImport(msg.to_string())
        } else {
            ErrorKind::Unknown(msg.to_string())
        }
    }

    pub fn generate_patches(diagnostics: &[CompilerDiagnostic]) -> Vec<RemediationPatch> {
        let mut patches = Vec::new();

        for diag in diagnostics {
            if !diag.file.exists() {
                continue;
            }

            if let Ok(content) = fs::read_to_string(&diag.file) {
                let lines: Vec<&str> = content.lines().collect();
                if diag.line > 0 && diag.line <= lines.len() {
                    let target_line = lines[diag.line - 1];

                    match &diag.error_kind {
                        ErrorKind::MissingMutability(_)
                            if target_line.contains("let ")
                                && !target_line.contains("let mut ") =>
                        {
                            let new_line = target_line.replacen("let ", "let mut ", 1);
                            patches.push(RemediationPatch {
                                file: diag.file.clone(),
                                line: diag.line,
                                old_text: target_line.to_string(),
                                new_text: new_line,
                            });
                        }
                        ErrorKind::MissingClone(_)
                            if !target_line.contains(".clone()") && target_line.ends_with(';') =>
                        {
                            let new_line =
                                target_line.trim_end_matches(';').to_string() + ".clone();";
                            patches.push(RemediationPatch {
                                file: diag.file.clone(),
                                line: diag.line,
                                old_text: target_line.to_string(),
                                new_text: new_line,
                            });
                        }
                        ErrorKind::MissingImport(symbol)
                            if symbol.contains("Path")
                                && !content.contains("use std::path::Path;") =>
                        {
                            patches.push(RemediationPatch {
                                file: diag.file.clone(),
                                line: 1,
                                old_text: String::new(),
                                new_text: "use std::path::Path;\n".to_string(),
                            });
                        }
                        _ => {}
                    }
                }
            }
        }

        patches
    }

    pub fn apply_patch(patch: &RemediationPatch) -> Result<(), std::io::Error> {
        if !patch.file.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Target file does not exist",
            ));
        }

        let content = fs::read_to_string(&patch.file)?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

        if patch.line == 1 && patch.old_text.is_empty() {
            lines.insert(0, patch.new_text.trim_end().to_string());
        } else if patch.line > 0 && patch.line <= lines.len() {
            lines[patch.line - 1] = patch.new_text.clone();
        }

        let mut updated = lines.join("\n");
        if content.ends_with('\n') {
            updated.push('\n');
        }

        fs::write(&patch.file, updated)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_parse_diagnostics_and_apply_mut_fix() {
        let temp = tempdir().unwrap();
        let src_file = temp.path().join("main.rs");
        fs::write(&src_file, "fn main() {\n    let x = 5;\n    x = 10;\n}\n").unwrap();

        let compiler_err = format!(
            "{}:2:5: error: cannot assign twice to immutable variable `x`",
            src_file.display()
        );
        let diagnostics = AutoRemediator::parse_diagnostics(&compiler_err);
        assert_eq!(diagnostics.len(), 1);

        let patches = AutoRemediator::generate_patches(&diagnostics);
        assert_eq!(patches.len(), 1);
        assert_eq!(patches[0].new_text, "    let mut x = 5;");

        AutoRemediator::apply_patch(&patches[0]).unwrap();
        let modified = fs::read_to_string(&src_file).unwrap();
        assert!(modified.contains("let mut x = 5;"));
    }
}
