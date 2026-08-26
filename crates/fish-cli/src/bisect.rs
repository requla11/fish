use std::path::Path;
use std::process::{Command, Stdio};

/// One commit from the scan window.
#[derive(Debug, Clone, PartialEq)]
pub struct CommitInfo {
    pub hash: String,
    pub subject: String,
}

/// Result of a full healing run.
#[derive(Debug, Clone, PartialEq)]
pub enum HealOutcome {
    /// The newest commit builds fine — nothing to do.
    NothingBroken,
    /// Every commit in the window failed; cannot isolate a culprit.
    NoGoodCommitWithinDepth { depth: usize },
    /// A revert branch was created locally and is ready for human review.
    RevertPrepared(Box<RevertPlan>),
}

/// A prepared, human-approved revert.
///
/// Fish never pushes or merges this — it only creates a local branch
/// containing one revert commit plus the PR text to open against dev.
#[derive(Debug, Clone, PartialEq)]
pub struct RevertPlan {
    pub branch: String,
    pub culprit: CommitInfo,
    pub pr_title: String,
    pub pr_body: String,
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

/// Newest-first list of commit hashes + subjects.
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

/// True when `git status --porcelain` reports any change.
pub fn working_tree_dirty(repo: &Path) -> std::io::Result<bool> {
    Ok(!git(repo, &["status", "--porcelain"])?.trim().is_empty())
}

/// Current branch name (falls back to short hash when detached).
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

/// Scan newest→oldest, invoking `run_build` once per commit until the
/// first pass. Returns the index of the newest passing commit.
pub fn find_first_good_index(
    commits: &[CommitInfo],
    mut run_build: impl FnMut() -> bool,
) -> Option<usize> {
    commits.iter().position(|_| run_build())
}

/// Pure helper producing the local branch name and PR copy for a
/// culprit commit. No side effects.
pub fn plan_revert(culprit: &CommitInfo) -> RevertPlan {
    let short: String = culprit.hash.chars().take(8).collect();
    let branch = format!("fish/revert-{short}");
    let pr_title = format!("Revert \"{}\"", culprit.subject);
    let pr_body = format!(
        "Automated revert prepared by `fish heal`.\n\n\
         Culprit: `{}` — {}\n\n\
         The build failed at HEAD but succeeded at this commit's parent.\n\
         Review carefully before merging; the revert may conflict with \
         later work.\n\nPublish with:\n\n    \
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
    }
}

fn checkout_detached(repo: &Path, hash: &str) -> std::io::Result<()> {
    git(repo, &["checkout", "-q", "--detach", hash]).map(|_| ())
}

/// `git -c user.name=… -c user.email=…` flags injected only when the
/// repository has no committer identity configured (fresh CI runners).
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

fn run_build_command(repo: &Path, words: &[String]) -> bool {
    match words.split_first() {
        Some((program, rest)) => Command::new(program)
            .args(rest)
            .current_dir(repo)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false),
        None => false,
    }
}

/// Full orchestration: verify clean tree, walk recent commits newest→oldest
/// running the build command at each detached HEAD, isolate the newest
/// failing commit, prepare a revert branch, and restore the original ref.
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

    let mut probe_idx = 0usize;
    let first_good = find_first_good_index(&commits, || {
        let commit = &commits[probe_idx];
        probe_idx += 1;
        checkout_detached(repo, &commit.hash)
            .map(|_| run_build_command(repo, build_words))
            .unwrap_or(false)
    });

    restore_ref(repo, &original)?;

    let Some(good_idx) = first_good else {
        return Ok(HealOutcome::NoGoodCommitWithinDepth { depth });
    };
    if good_idx == 0 {
        return Ok(HealOutcome::NothingBroken);
    }

    let culprit = commits[good_idx - 1].clone();
    let plan = Box::new(plan_revert(&culprit));

    git(repo, &["checkout", "-q", "-b", &plan.branch])?;
    let mut rev_args: Vec<String> = git_author_flags(repo);
    rev_args.extend(["revert".into(), "--no-edit".into(), culprit.hash.clone()]);
    let rev_refs: Vec<&str> = rev_args.iter().map(String::as_str).collect();
    let reverted = git(repo, &rev_refs);
    if let Err(e) = reverted {
        // Clean up the conflicted branch attempt and get out safely.
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

    /// Same as above but without committing the canary, so every commit
    /// fails the probe command (depth-exhaustion scenario).
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

    /// Probe succeeds iff `canary.txt` is tracked at the checked-out
    /// commit. Pure git — identical behaviour on every platform.
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

        // Back on master/main, and the revert branch exists.
        let branch = current_ref(repo).unwrap();
        assert_ne!(branch, plan.branch);
        let branches = git(repo, &["branch", "--list", &plan.branch]).unwrap();
        assert!(!branches.trim().is_empty());

        // Switching to the branch shows a revert commit restoring the canary.
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
        // No canary ever committed → the probe fails at every commit.
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
