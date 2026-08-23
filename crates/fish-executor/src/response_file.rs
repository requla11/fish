use std::fs;
use std::path::{Path, PathBuf};

use crate::command::CommandSpec;

pub struct ResponseFileWriter;

impl ResponseFileWriter {
    pub const WINDOWS_MAX_CMD_LEN: usize = 4096;
    pub const POSIX_MAX_CMD_LEN: usize = 32768;

    pub fn should_use_response_file(args: &[String]) -> bool {
        let total_len: usize = args.iter().map(|a| a.len() + 1).sum();
        let limit = if cfg!(windows) {
            Self::WINDOWS_MAX_CMD_LEN
        } else {
            Self::POSIX_MAX_CMD_LEN
        };
        total_len > limit || args.len() > 100
    }

    pub fn write_response_file(dir: &Path, args: &[String]) -> std::io::Result<PathBuf> {
        fs::create_dir_all(dir)?;
        let hash = fastrand::u64(..);
        let path = dir.join(format!("fish_args_{hash:016x}.rsp"));
        let mut content = String::new();
        for arg in args {
            if arg.contains('\n') || arg.contains('\r') {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "argument contains a line break and cannot be represented in a response file",
                ));
            }
            if arg.contains(' ') || arg.contains('\t') || arg.contains('"') {
                quote_arg(arg, &mut content);
            } else {
                content.push_str(arg);
            }
            content.push('\n');
        }
        fs::write(&path, content)?;
        Ok(path)
    }

    pub fn adapt_command_for_response_file(
        spec: &mut CommandSpec,
        temp_dir: &Path,
    ) -> std::io::Result<Option<PathBuf>> {
        if !Self::should_use_response_file(&spec.args) {
            return Ok(None);
        }
        let rsp_path = Self::write_response_file(temp_dir, &spec.args)?;
        spec.args.clear();
        spec.args.push(format!("@{}", rsp_path.to_string_lossy()));
        Ok(Some(rsp_path))
    }
}

/// Quote an argument using Windows command-line rules: only backslash runs
/// immediately preceding a double quote are doubled, and embedded quotes are
/// emitted as `\"`. Backslashes elsewhere stay untouched so paths, regexes,
/// and defines survive verbatim.
fn quote_arg(arg: &str, out: &mut String) {
    out.push('"');
    let mut backslashes = 0usize;
    for c in arg.chars() {
        match c {
            '\\' => backslashes += 1,
            '"' => {
                for _ in 0..backslashes * 2 + 1 {
                    out.push('\\');
                }
                out.push('"');
                backslashes = 0;
            }
            _ => {
                for _ in 0..backslashes {
                    out.push('\\');
                }
                out.push(c);
                backslashes = 0;
            }
        }
    }
    for _ in 0..backslashes * 2 {
        out.push('\\');
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_should_use_response_file_thresholds() {
        let small_args = vec!["--flag".to_string(), "target".to_string()];
        assert!(!ResponseFileWriter::should_use_response_file(&small_args));

        let large_args: Vec<String> = (0..150).map(|i| format!("arg_{i}")).collect();
        assert!(ResponseFileWriter::should_use_response_file(&large_args));
    }

    #[test]
    fn test_write_response_file_and_adapt() {
        let temp = tempdir().unwrap();
        let mut spec = CommandSpec::new("rustc");
        for i in 0..200 {
            spec.args.push(format!("--extern=crate_{i}"));
        }
        let rsp = ResponseFileWriter::adapt_command_for_response_file(&mut spec, temp.path())
            .unwrap()
            .expect("should create rsp");

        assert!(rsp.exists());
        assert_eq!(spec.args.len(), 1);
        assert!(spec.args[0].starts_with('@'));

        let content = fs::read_to_string(&rsp).unwrap();
        assert!(content.contains("--extern=crate_0"));
        assert!(content.contains("--extern=crate_199"));
    }

    #[test]
    fn test_quote_arg_preserves_path_backslashes() {
        let mut out = String::new();
        quote_arg(r"C:\Program Files\tool chain", &mut out);
        assert_eq!(out, "\"C:\\Program Files\\tool chain\"");
    }

    #[test]
    fn test_quote_arg_doubles_backslashes_only_before_quotes() {
        let mut out = String::new();
        quote_arg("a\\\"b", &mut out);
        assert_eq!(out, "\"a\\\\\\\"b\"");

        let mut trailing = String::new();
        quote_arg("ends with backslash \\", &mut trailing);
        assert_eq!(trailing, "\"ends with backslash \\\\\"");
    }

    #[test]
    fn test_response_file_rejects_embedded_newlines() {
        let temp = tempdir().unwrap();
        let args = vec!["--flag".to_string(), "bad\narg".to_string()];
        let result = ResponseFileWriter::write_response_file(temp.path(), &args);
        assert!(result.is_err(), "embedded newlines must be rejected");
    }
}
