use std::path::Path;
use std::process::{Command, Stdio};

use crate::self_heal;

#[derive(Debug, Clone, PartialEq)]
pub struct CommitInfo {
    pub hash: String,
    pub subject: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealOutcome {
    NothingBroken,
    NoGoodCommitWithinDepth { depth: usize },
    RevertPrepared(Box<RevertPlan>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct RevertPlan {
    pub branch: String,
    pub culprit: CommitInfo,
    pub pr_title: String,
    pub pr_body: String,
    pub suggestions: Vec<self_heal::RepairSuggestion>,
}

fn git(repo: &Path, args: &[&str]) -> std::io::Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo)
        .stderr(Stdio::piped())
        .stdout(Stdio::piped())
        .output()?;
    if !out.status.success() {
        return Err(std::io::Error::other(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn list_recent_commits(repo: &Path, depth: usize) -> std::io::Result<Vec<CommitInfo>> {
    let out = git(repo, &["log", &format!("-n{depth}"), "--format=%H %s"])?;
    Ok(out
        .lines()
        .filter_map(|line| {
            line.split_once(' ').map(|(h, s)| CommitInfo {
                hash: h.trim().to_string(),
                subject: s.to_string(),
            })
        })
        .collect())
}

pub fn working_tree_dirty(repo: &Path) -> std::io::Result<bool> {
    Ok(!git(repo, &["status", "--porcelain"])?.trim().is_empty())
}

pub fn current_ref(repo: &Path) -> std::io::Result<String> {
    let out = git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])?;
    let name = out.trim();
    if name == "HEAD" {
        let short = git(repo, &["rev-parse", "--short", "HEAD"])?;
        Ok(short.trim().to_string())
    } else {
        Ok(name.to_string())
    }
}

#[allow(dead_code)]
pub fn find_first_good_index(
    commits: &[CommitInfo],
    mut run_build: impl FnMut() -> bool,
) -> Option<usize> {
    commits.iter().position(|_| run_build())
}

pub fn bisect_binary_search(
    commits: &[CommitInfo],
    mut run_build: impl FnMut(usize) -> bool,
) -> Option<usize> {
    if commits.is_empty() {
        return None;
    }
    if run_build(0) {
        return Some(0);
    }
    let last = commits.len() - 1;
    if !run_build(last) {
        return None;
    }

    let mut low = 0;
    let mut high = last;
    while low + 1 < high {
        let mid = low + (high - low) / 2;
        if run_build(mid) {
            high = mid;
        } else {
            low = mid;
        }
    }
    Some(high)
}

#[allow(dead_code)]
pub fn plan_revert(culprit: &CommitInfo) -> RevertPlan {
    plan_revert_with_diagnostics(culprit, &[])
}

pub fn plan_revert_with_diagnostics(
    culprit: &CommitInfo,
    suggestions: &[self_heal::RepairSuggestion],
) -> RevertPlan {
    let short: String = culprit.hash.chars().take(8).collect();
    let branch = format!("fish/revert-{short}");
    let pr_title = format!("Revert \"{}\"", culprit.subject);

    let mut diagnostics_section = String::new();
    if !suggestions.is_empty() {
        diagnostics_section.push_str("\n### Automated Root-Cause Analysis\n");
        for sug in suggestions {
            diagnostics_section.push_str(&format!(
                "- **Category**: `{}`\n  - **Match**: `{}`\n  - **Advice**: {}\n",
                sug.category, sug.matched_line, sug.advice
            ));
        }
    }

    let pr_body = format!(
        "Automated revert prepared by `fish heal`.\n\n\
         Culprit: `{}` — {}\n\n\
         The build failed at HEAD but succeeded at this commit's parent.\n\
         Review carefully before merging; the revert may conflict with later work.{diagnostics_section}\n\n\
         Publish with:\n\n    \
         git push -u origin {branch}\n    \
         gh pr create --fill",
        culprit.hash,
        culprit.subject,
        branch = branch
    );
    RevertPlan {
        branch,
        culprit: culprit.clone(),
        pr_title,
        pr_body,
        suggestions: suggestions.to_vec(),
    }
}

fn checkout_detached(repo: &Path, hash: &str) -> std::io::Result<()> {
    git(repo, &["checkout", "-q", "--detach", hash]).map(|_| ())
}

fn git_author_flags(repo: &Path) -> Vec<String> {
    let configured = Command::new("git")
        .args(["config", "user.email"])
        .current_dir(repo)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .map(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false);
    if configured {
        Vec::new()
    } else {
        vec![
            "-c".into(),
            "user.name=fish-heal".into(),
            "-c".into(),
            "user.email=fish@localhost".into(),
        ]
    }
}

fn restore_ref(repo: &Path, reference: &str) -> std::io::Result<()> {
    git(repo, &["checkout", "-q", reference]).map(|_| ())
}

fn run_build_command_captured(repo: &Path, words: &[String]) -> (bool, String) {
    match words.split_first() {
        Some((program, rest)) => match Command::new(program)
            .args(rest)
            .current_dir(repo)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
        {
            Ok(output) => {
                let success = output.status.success();
                let combined = format!(
                    "{}\n{}",
                    String::from_utf8_lossy(&output.stdout),
                    String::from_utf8_lossy(&output.stderr)
                );
                (success, combined)
            }
            Err(e) => (false, e.to_string()),
        },
        None => (false, String::new()),
    }
}

pub fn heal(repo: &Path, depth: usize, build_words: &[String]) -> std::io::Result<HealOutcome> {
    if working_tree_dirty(repo)? {
        return Err(std::io::Error::other(
            "working tree has uncommitted changes — commit or stash them before `fish heal`",
        ));
    }

    let original = current_ref(repo)?;
    let commits = list_recent_commits(repo, depth)?;
    if commits.is_empty() {
        return Err(std::io::Error::other("repository has no commits"));
    }

    let mut last_failure_output = String::new();
    let first_good = bisect_binary_search(&commits, |idx| {
        let commit = &commits[idx];
        if checkout_detached(repo, &commit.hash).is_ok() {
            let (success, out) = run_build_command_captured(repo, build_words);
            if !success && idx == 0 {
                last_failure_output = out;
            }
            success
        } else {
            false
        }
    });

    restore_ref(repo, &original)?;

    let Some(good_idx) = first_good else {
        return Ok(HealOutcome::NoGoodCommitWithinDepth { depth });
    };
    if good_idx == 0 {
        return Ok(HealOutcome::NothingBroken);
    }

    let culprit = commits[good_idx - 1].clone();
    let suggestions = self_heal::analyze_failure(&last_failure_output);
    let plan = Box::new(plan_revert_with_diagnostics(&culprit, &suggestions));

    git(repo, &["checkout", "-q", "-b", &plan.branch])?;
    let mut rev_args: Vec<String> = git_author_flags(repo);
    rev_args.extend(["revert".into(), "--no-edit".into(), culprit.hash.clone()]);
    let rev_refs: Vec<&str> = rev_args.iter().map(String::as_str).collect();
    let reverted = git(repo, &rev_refs);
    if let Err(e) = reverted {
        let _ = git(repo, &["revert", "--abort"]);
        let _ = restore_ref(repo, &original);
        let _ = git(repo, &["branch", "-D", &plan.branch]);
        return Err(std::io::Error::other(format!(
            "revert of {} conflicted; cleaned up branch {}. {}",
            culprit.hash, plan.branch, e
        )));
    }
    restore_ref(repo, &original)?;

    Ok(HealOutcome::RevertPrepared(plan))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn commit_info(hash: &str, subject: &str) -> CommitInfo {
        CommitInfo {
            hash: hash.to_string(),
            subject: subject.to_string(),
        }
    }

    #[test]
    fn plan_revert_formats_pr_copy() {
        let plan = plan_revert(&commit_info("abcdef1234567890", "break the build"));
        assert_eq!(plan.branch, "fish/revert-abcdef12");
        assert_eq!(plan.pr_title, "Revert \"break the build\"");
        assert!(plan.pr_body.contains("`abcdef1234567890`"));
        assert!(
            plan.pr_body
                .contains("git push -u origin fish/revert-abcdef12")
        );
    }

    #[test]
    fn plan_revert_includes_diagnostics() {
        let suggestions = vec![self_heal::RepairSuggestion {
            category: "missing-symbol",
            matched_line: "cannot find function `foo` in crate `bar`".into(),
            advice: "Check feature gates".into(),
        }];
        let plan = plan_revert_with_diagnostics(
            &commit_info("abcdef1234567890", "break the build"),
            &suggestions,
        );
        assert!(plan.pr_body.contains("Automated Root-Cause Analysis"));
        assert!(plan.pr_body.contains("missing-symbol"));
    }

    #[test]
    fn test_bisect_binary_search_finds_exact_boundary() {
        let commits = vec![
            commit_info("c5", "failing head"),
            commit_info("c4", "failing"),
            commit_info("c3", "failing"),
            commit_info("c2", "passing"),
            commit_info("c1", "passing base"),
        ];

        let idx = bisect_binary_search(&commits, |i| i >= 3);
        assert_eq!(idx, Some(3));
    }

    #[test]
    fn find_first_good_scans_newest_to_oldest() {
        let commits = vec![
            commit_info("c3", "newest"),
            commit_info("c2", "middle"),
            commit_info("c1", "oldest"),
        ];
        let mut calls = Vec::new();
        let idx = find_first_good_index(&commits, || {
            calls.push(());
            calls.len() >= 2
        });
        assert_eq!(idx, Some(1));
        assert_eq!(calls.len(), 2);
    }

    #[test]
    fn find_first_good_none_when_all_fail() {
        let commits = vec![commit_info("b", "x"), commit_info("a", "y")];
        assert_eq!(find_first_good_index(&commits, || false), None);
    }

    #[test]
    fn find_first_good_immediate_pass_is_head() {
        let commits = vec![commit_info("b", "x"), commit_info("a", "y")];
        assert_eq!(find_first_good_index(&commits, || true), Some(0));
    }

    fn git_init_with_canary(dir: &Path) {
        let run = |args: &[&str]| {
            Command::new("git")
                .args(["-c", "user.email=fish@test", "-c", "user.name=fish"])
                .args(args)
                .current_dir(dir)
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("git")
        };
        run(&["init"]);
        std::fs::write(dir.join("canary.txt"), "ok").unwrap();
        run(&["add", "."]);
        run(&["commit", "-m", "base with canary"]);
    }

    fn git_init_without_canary(dir: &Path) {
        Command::new("git")
            .args(["-c", "user.email=fish@test", "-c", "user.name=fish"])
            .args(["init"])
            .current_dir(dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git");
        std::fs::write(dir.join("readme.md"), "x").unwrap();
        Command::new("git")
            .args(["-c", "user.email=fish@test", "-c", "user.name=fish"])
            .args(["add", "."])
            .current_dir(dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git");
        Command::new("git")
            .args(["-c", "user.email=fish@test", "-c", "user.name=fish"])
            .args(["commit", "-m", "base without canary"])
            .current_dir(dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("git");
    }

    fn build_cmd_for() -> Vec<String> {
        vec!["git".into(), "show".into(), "HEAD:canary.txt".into()]
    }

    fn remove_canary_commit(dir: &Path, msg: &str) {
        std::fs::remove_file(dir.join("canary.txt")).unwrap();
        Command::new("git")
            .args(["-c", "user.email=fish@test", "-c", "user.name=fish"])
            .args(["commit", "-am", msg])
            .current_dir(dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
    }

    fn add_empty_commit(dir: &Path, msg: &str) {
        Command::new("git")
            .args(["-c", "user.email=fish@test", "-c", "user.name=fish"])
            .args(["commit", "--allow-empty", "-m", msg])
            .current_dir(dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap();
    }

    #[test]
    fn heal_prepares_revert_branch_and_restores_original() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        git_init_with_canary(repo);

        remove_canary_commit(repo, "delete canary");
        add_empty_commit(repo, "unrelated work");

        let build = build_cmd_for();
        let outcome = heal(repo, 10, &build).expect("heal");

        let HealOutcome::RevertPrepared(plan) = outcome else {
            panic!("expected RevertPrepared, got {outcome:?}");
        };
        assert!(plan.culprit.subject.contains("delete canary"));

        let branch = current_ref(repo).unwrap();
        assert_ne!(branch, plan.branch);
        let branches = git(repo, &["branch", "--list", &plan.branch]).unwrap();
        assert!(!branches.trim().is_empty());

        restore_ref(repo, &plan.branch).unwrap();
        assert!(repo.join("canary.txt").exists());
    }

    #[test]
    fn heal_reports_nothing_broken_when_head_passes() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        git_init_with_canary(repo);
        let build = build_cmd_for();
        assert_eq!(heal(repo, 10, &build).unwrap(), HealOutcome::NothingBroken);
    }

    #[test]
    fn heal_all_fail_reports_depth_exhausted() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        git_init_without_canary(repo);
        add_empty_commit(repo, "more work without canary");
        let build = build_cmd_for();
        assert_eq!(
            heal(repo, 5, &build).unwrap(),
            HealOutcome::NoGoodCommitWithinDepth { depth: 5 }
        );
    }

    #[test]
    fn heal_refuses_dirty_tree() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path();
        git_init_with_canary(repo);
        std::fs::write(repo.join("dirty.txt"), "x").unwrap();
        assert!(heal(repo, 5, &[]).is_err());
    }
}
