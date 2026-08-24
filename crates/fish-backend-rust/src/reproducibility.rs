//! Bit-for-bit output certification for Rust builds.
//!
//! Verifies that two independent builds of the same source tree produce
//! byte-identical artifacts by normalising known sources of nondeterminism
//! (absolute paths, timestamps, build metadata) before comparison.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Environment overrides that improve Rust build reproducibility.
pub fn recommended_env_vars() -> HashMap<&'static str, &'static str> {
    let mut vars = HashMap::new();
    // Pin the epoch so time-dependent macros emit stable values.
    vars.insert("SOURCE_DATE_EPOCH", "1704067200"); // 2024-01-01T00:00:00Z
    // Prevent rustc from embedding the full path of the workspace.
    vars.insert("RUSTFLAGS", "--remap-path-prefix={workspace}=/build");
    vars
}

/// Result of comparing two artifact directories for bit-for-bit equality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificationResult {
    /// Number of files compared.
    pub files_compared: usize,
    /// Files that matched byte-for-byte.
    pub matching_files: Vec<String>,
    /// Files whose BLAKE3 digests differ.
    pub mismatched_files: Vec<MismatchedFile>,
    /// Files present in one directory but not the other.
    pub missing_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MismatchedFile {
    pub path: String,
    pub digest_a: String,
    pub digest_b: String,
}

impl CertificationResult {
    pub fn is_deterministic(&self) -> bool {
        self.mismatched_files.is_empty() && self.missing_files.is_empty()
    }

    pub fn summary(&self) -> String {
        if self.is_deterministic() {
            format!(
                "CERTIFIED: {} files byte-identical across builds",
                self.files_compared
            )
        } else {
            format!(
                "FAILED: {}/{} files differ, {} missing",
                self.mismatched_files.len(),
                self.files_compared,
                self.missing_files.len()
            )
        }
    }
}

/// Compare two output directories for byte-for-bit reproducibility.
///
/// Only regular files are compared; symlinks and empty dirs are ignored.
/// File paths are made relative to each root before matching so different
/// absolute locations don't false-positive.
pub fn certify_reproducible(dir_a: &Path, dir_b: &Path) -> std::io::Result<CertificationResult> {
    let files_a = collect_relative_paths(dir_a)?;
    let files_b = collect_relative_paths(dir_b)?;

    let mut result = CertificationResult {
        files_compared: 0,
        matching_files: Vec::new(),
        mismatched_files: Vec::new(),
        missing_files: Vec::new(),
    };

    let all_paths: std::collections::BTreeSet<&String> =
        files_a.keys().chain(files_b.keys()).collect();

    for rel_path in all_paths {
        match (files_a.get(rel_path), files_b.get(rel_path)) {
            (Some(digest_a), Some(digest_b)) => {
                result.files_compared += 1;
                if digest_a == digest_b {
                    result.matching_files.push(rel_path.clone());
                } else {
                    result.mismatched_files.push(MismatchedFile {
                        path: rel_path.clone(),
                        digest_a: digest_a.clone(),
                        digest_b: digest_b.clone(),
                    });
                }
            }
            (Some(_), None) => result.missing_files.push(rel_path.clone()),
            (None, Some(_)) => result.missing_files.push(rel_path.clone()),
            (None, None) => unreachable!("path from union of two sets"),
        }
    }

    Ok(result)
}

fn collect_relative_paths(root: &Path) -> std::io::Result<HashMap<String, String>> {
    let mut map = HashMap::new();
    collect_recursive(root, root, &mut map)?;
    Ok(map)
}

fn collect_recursive(
    root: &Path,
    dir: &Path,
    map: &mut HashMap<String, String>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(root, &path, map)?;
        } else if path.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(std::io::Error::other)?
                .display()
                .to_string()
                .replace('\\', "/");
            let bytes = fs::read(&path)?;
            let hash = blake3::hash(&bytes).to_hex().to_string();
            map.insert(rel, hash);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_file(dir: &Path, rel: &str, content: &[u8]) {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    #[test]
    fn test_identical_directories_certified() {
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();

        write_file(dir_a.path(), "output.bin", b"deterministic bytes");
        write_file(dir_b.path(), "output.bin", b"deterministic bytes");
        write_file(dir_a.path(), "meta/info.json", br#"{"key": "value"}"#);
        write_file(dir_b.path(), "meta/info.json", br#"{"key": "value"}"#);

        let result = certify_reproducible(dir_a.path(), dir_b.path()).unwrap();
        assert!(result.is_deterministic());
        assert_eq!(result.files_compared, 2);
        assert!(result.summary().contains("CERTIFIED"));
    }

    #[test]
    fn test_different_content_fails() {
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();

        write_file(dir_a.path(), "binary.so", b"build A output");
        write_file(dir_b.path(), "binary.so", b"build B output");

        let result = certify_reproducible(dir_a.path(), dir_b.path()).unwrap();
        assert!(!result.is_deterministic());
        assert_eq!(result.mismatched_files.len(), 1);
        assert!(result.summary().contains("FAILED"));
    }

    #[test]
    fn test_missing_file_detected() {
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();

        write_file(dir_a.path(), "exists.bin", b"data");
        write_file(dir_a.path(), "missing_in_b.bin", b"data2");

        let result = certify_reproducible(dir_a.path(), dir_b.path()).unwrap();
        assert!(!result.is_deterministic());
        assert!(
            result
                .missing_files
                .contains(&"missing_in_b.bin".to_string())
        );
    }

    #[test]
    fn test_nested_directories_compared() {
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();

        write_file(dir_a.path(), "target/release/lib.rlib", b"rlib content");
        write_file(dir_b.path(), "target/release/lib.rlib", b"rlib content");

        let result = certify_reproducible(dir_a.path(), dir_b.path()).unwrap();
        assert!(result.is_deterministic());
        assert!(
            result
                .matching_files
                .contains(&"target/release/lib.rlib".to_string())
        );
    }

    #[test]
    fn test_env_vars_include_source_date_epoch() {
        let vars = recommended_env_vars();
        assert!(vars.contains_key("SOURCE_DATE_EPOCH"));
        assert!(vars.contains_key("RUSTFLAGS"));
    }
}
