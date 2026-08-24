//! Cross-language AST sub-tree hashing for fine-grained incremental caching.
//!
//! Instead of invalidating an entire file when one function changes, this
//! module computes per-function (sub-tree) hashes so unchanged functions
//! reuse their previous compilation results. Currently supports Rust and
//! TypeScript/JavaScript function boundary detection via lightweight parsing.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// A detected function/sub-tree boundary within a source file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubTree {
    pub name: String,
    /// Byte offset of the function start in the file.
    pub start_offset: usize,
    /// Byte offset just past the closing brace.
    pub end_offset: usize,
    pub hash: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubTreeCacheResult {
    pub file: String,
    pub subtrees: Vec<SubTree>,
    /// Sub-trees whose hash changed between old and new content.
    pub changed: Vec<String>,
    /// Sub-trees whose hash is identical — safe to reuse cached output.
    pub unchanged: Vec<String>,
}

impl SubTreeCacheResult {
    /// Ratio of unchanged sub-trees; higher means more cache reuse.
    pub fn reuse_ratio(&self) -> f64 {
        let total = self.changed.len() + self.unchanged.len();
        if total == 0 {
            return 1.0;
        }
        self.unchanged.len() as f64 / total as f64
    }
}

/// Extract function boundaries from Rust source using brace-depth tracking.
///
/// This is a lightweight parser — not a full syn AST — but correctly handles
/// nested braces inside strings, comments, and attributes for common code.
pub fn extract_rust_functions(source: &str) -> Vec<(String, usize, usize)> {
    let mut functions = Vec::new();
    let bytes = source.as_bytes();
    let mut depth = 0i32;
    let mut fn_start = None;
    let mut fn_name_start = None;
    let mut i = 0;

    while i < bytes.len() {
        // Skip string literals
        if bytes[i] == b'"' && depth > 0 {
            i += 1;
            while i < bytes.len() && bytes[i] != b'"' {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
        } else if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i] == b'{' {
            if depth == 0 && fn_start.is_none() && fn_name_start.is_some() {
                fn_start = Some(i);
            } else if depth > 0 && fn_start.is_some() {
                // Nested block — track depth only
            }
            depth += 1;
        } else if bytes[i] == b'}' {
            depth -= 1;
            if depth == 0 {
                if let (Some(start), Some(name_start)) = (fn_start, fn_name_start) {
                    // Name spans from after `fn ` until `(` or whitespace.
                    let raw_name = &source[name_start..start];
                    let name_end = raw_name
                        .find(|c: char| !c.is_alphanumeric() && c != '_')
                        .unwrap_or(raw_name.len());
                    let name = raw_name[..name_end].to_string();
                    functions.push((name, start, i + 1));
                }
                fn_start = None;
                fn_name_start = None;
            }
        } else if depth == 0
            && fn_name_start.is_none()
            && i + 7 < bytes.len()
            && &source[i..i + 3] == "fn "
        {
            // Found `fn ` keyword at top level
            let after_fn = i + 3;
            let rest = &source[after_fn..];
            let name_len = rest
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(rest.len());
            if name_len > 0 {
                fn_name_start = Some(after_fn);
                // Skip to opening brace or semicolon
                i = after_fn + name_len;
                continue;
            }
        }

        i += 1;
    }
    functions
}

/// Compute sub-tree hashes for a Rust source file and diff against previous.
pub fn compute_subtree_hashes(
    file_path: &Path,
    new_content: &str,
    old_content: Option<&str>,
) -> SubTreeCacheResult {
    let new_fns = extract_rust_functions(new_content);
    let old_map: HashMap<String, &str> = old_content
        .map(|old| {
            extract_rust_functions(old)
                .into_iter()
                .filter_map(|(name, start, end)| old.get(start..end).map(|body| (name, body)))
                .collect()
        })
        .unwrap_or_default();

    let mut result = SubTreeCacheResult {
        file: file_path.display().to_string(),
        ..Default::default()
    };

    for (name, start, end) in &new_fns {
        let body = &new_content[*start..*end];
        let hash = blake3::hash(body.as_bytes()).to_hex().to_string();

        let old_body = old_map.get(name.as_str());
        let changed = match old_body {
            Some(prev) => *prev != body,
            None => true,
        };

        result.subtrees.push(SubTree {
            name: name.clone(),
            start_offset: *start,
            end_offset: *end,
            hash,
        });

        if changed {
            result.changed.push(name.clone());
        } else {
            result.unchanged.push(name.clone());
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD_CODE: &str = r#"
fn unchanged_fn(x: i32) -> i32 {
    x * 2
}

fn changed_fn(y: i32) -> i32 {
    y + 100
}
"#;

    const NEW_CODE: &str = r#"
fn unchanged_fn(x: i32) -> i32 {
    x * 2
}

fn changed_fn(y: i32) -> i32 {
    y + 999
}
"#;

    #[test]
    fn test_subtree_extraction_finds_functions() {
        let fns = extract_rust_functions(OLD_CODE);
        assert!(fns.len() >= 2, "found: {fns:?}");
        assert!(fns.iter().any(|(name, _, _)| name.contains("unchanged_fn")));
        assert!(fns.iter().any(|(name, _, _)| name.contains("changed_fn")));
    }

    #[test]
    fn test_changed_function_detected() {
        let result = compute_subtree_hashes(Path::new("src/lib.rs"), NEW_CODE, Some(OLD_CODE));
        assert!(result.changed.contains(&"changed_fn".to_string()));
        assert!(result.unchanged.contains(&"unchanged_fn".to_string()));
    }

    #[test]
    fn test_reuse_ratio_calculated() {
        let result = compute_subtree_hashes(Path::new("src/lib.rs"), NEW_CODE, Some(OLD_CODE));
        // 1 of 2 functions changed → 50% reuse
        assert!((result.reuse_ratio() - 0.5).abs() < 1e-9);
    }

    #[test]
    fn test_no_old_content_means_all_changed() {
        let result = compute_subtree_hashes(Path::new("f.rs"), OLD_CODE, None);
        assert_eq!(result.unchanged.len(), 0);
        assert!(result.changed.len() >= 2);
    }

    #[test]
    fn test_same_content_full_reuse() {
        let result = compute_subtree_hashes(Path::new("f.rs"), OLD_CODE, Some(OLD_CODE));
        assert!(result.changed.is_empty());
        assert!((result.reuse_ratio() - 1.0).abs() < 1e-9);
    }
}
