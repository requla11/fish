use std::path::Path;
use std::process::{Command, Stdio};

/// A single actionable repair suggestion derived from build output.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct RepairSuggestion {
    pub category: &'static str,
    pub matched_line: String,
    pub advice: String,
}

/// Analyze captured build stderr/stdout for known failure classes.
///
/// Pure function over text — no process spawning — so it is cheap to
/// call on every failed build and fully unit-testable.
pub fn analyze_failure(output: &str) -> Vec<RepairSuggestion> {
    let mut suggestions = Vec::new();
    let mut seen_categories: Vec<&'static str> = Vec::new();
    let mut push = |s: RepairSuggestion, seen: &mut Vec<&'static str>| {
        if !seen.contains(&s.category) {
            seen.push(s.category);
            suggestions.push(s);
        }
    };

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.contains("unresolved import") || trimmed.contains("cannot find") {
            push(
                RepairSuggestion {
                    category: "missing-symbol",
                    matched_line: trimmed.to_string(),
                    advice: "Check spelling/feature gates; run `fish why <symbol>` or grep the crate exports. If a dependency feature is required, enable it in Cargo.toml.".into(),
                },
                &mut seen_categories,
            );
        }

        if (trimmed.starts_with("error:") && trimmed.contains("linked"))
            || trimmed.contains("linker") && trimmed.contains("failed")
        {
            push(
                RepairSuggestion {
                    category: "linker",
                    matched_line: trimmed.to_string(),
                    advice: "Linker failure. On Windows MSVC ensure VS Build Tools are installed; on Linux install system libs referenced in the error (e.g. pkg-config, -lssl).".into(),
                },
                &mut seen_categories,
            );
        }

        if trimmed.contains("no matching package named") {
            push(
                RepairSuggestion {
                    category: "missing-dep",
                    matched_line: trimmed.to_string(),
                    advice: "Dependency not found in any registry. Verify the name/version in Cargo.toml; run `cargo update` if the lockfile is stale.".into(),
                },
                &mut seen_categories,
            );
        }

        if trimmed.contains("version solving failed")
            || trimmed.contains("conflicting requirements")
        {
            push(
                RepairSuggestion {
                    category: "version-conflict",
                    matched_line: trimmed.to_string(),
                    advice: "Two crates require incompatible versions of a shared dependency. Add an explicit `=x.y.z` pin or use `cargo update -p <crate> --precise`.".into(),
                },
                &mut seen_categories,
            );
        }

        if trimmed.contains("permission denied") || trimmed.contains("Access is denied") {
            push(
                RepairSuggestion {
                    category: "permissions",
                    matched_line: trimmed.to_string(),
                    advice: "Filesystem permission issue — check that target/ is writable and no other fish/cargo process holds a lock.".into(),
                },
                &mut seen_categories,
            );
        }

        if trimmed.contains("out of memory") || trimmed.contains("OOMKilled") {
            push(
                RepairSuggestion {
                    category: "oom",
                    matched_line: trimmed.to_string(),
                    advice: "Task exceeded its memory budget. Reduce parallelism (`-j`) or raise the per-task limit via the resource governor profile.".into(),
                },
                &mut seen_categories,
            );
        }
    }
    suggestions
}

/// Attempt automatic repairs for a Rust project by delegating to
/// `cargo fix --allow-dirty`. Returns captured output when attempted.
pub fn attempt_cargo_auto_fix(project_dir: &Path) -> Option<std::io::Result<String>> {
    if !project_dir.join("Cargo.toml").exists() {
        return None;
    }
    let out = Command::new("cargo")
        .args(["fix", "--allow-dirty", "--allow-staged"])
        .current_dir(project_dir)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output();
    Some(out.map(|o| {
        format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_output_no_suggestions() {
        assert!(analyze_failure("").is_empty());
    }

    #[test]
    fn detects_missing_symbol() {
        let out = analyze_failure("error[E0432]: unresolved import `foo::bar`");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].category, "missing-symbol");
    }

    #[test]
    fn detects_linker_failure() {
        let out = analyze_failure("error: linker `link.exe` failed: exit code 1181");
        assert_eq!(out[0].category, "linker");
    }

    #[test]
    fn dedupes_same_category() {
        let out =
            analyze_failure("error[E0432]: unresolved import `a`\nerror[E0433]: cannot find `b`\n");
        assert_eq!(out.len(), 1);
    }

    #[test]
    fn detects_oom_and_permissions() {
        let mut out = analyze_failure("fatal: out of memory");
        assert_eq!(out[0].category, "oom");
        out = analyze_failure("cp: permission denied");
        assert_eq!(out[0].category, "permissions");
    }

    #[test]
    fn multiple_categories_reported() {
        let out = analyze_failure("error[E0432]: unresolved import `x`\nerror: linker failed\n");
        assert_eq!(out.len(), 2);
        let cats: Vec<_> = out.iter().map(|s| s.category).collect();
        assert!(cats.contains(&"missing-symbol"));
        assert!(cats.contains(&"linker"));
    }

    #[test]
    fn auto_fix_skips_non_cargo_dirs() {
        let dir = tempfile::tempdir().unwrap();
        assert!(attempt_cargo_auto_fix(dir.path()).is_none());
    }
}
