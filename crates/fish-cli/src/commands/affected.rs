use std::path::{Path, PathBuf};
use std::process::ExitCode;

use fish_backend_rust::BuildMode;
use fish_core::project::Project;

use crate::args::AffectedArgs;
use crate::build;
use crate::config::{BackendChoice, FishConfig};
use crate::utils::resolve_start_dir;

pub fn run_affected(args: AffectedArgs) -> ExitCode {
    let mode = args.mode.to_build_mode();
    let start_dir = match resolve_start_dir(args.common.path.as_deref()) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let config = match FishConfig::load(&start_dir) {
        Ok(Some(config)) => config,
        Ok(None) => FishConfig::default(),
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };
    if !matches!(config.backend, BackendChoice::Auto | BackendChoice::Rust) {
        eprintln!(
            "error: `fish affected` only supports Rust workspaces (backend `{:?}` configured)",
            config.backend
        );
        return ExitCode::FAILURE;
    }

    let project = match Project::discover(&start_dir) {
        Ok(Some(project)) => project,
        Ok(None) => {
            eprintln!(
                "error: no Cargo project found in `{}` or any parent directory",
                start_dir.display()
            );
            return ExitCode::FAILURE;
        }
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("hint: make sure `cargo` is installed and available on PATH");
            return ExitCode::FAILURE;
        }
    };

    let repo_root = match git_output(&start_dir, &["rev-parse", "--show-toplevel"]) {
        Ok(root) => PathBuf::from(root.trim()),
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let git_ref = args.base.as_deref().unwrap_or(&args.since);

    let mut changed = Vec::new();
    if let Err(message) = collect_git_changes(&repo_root, git_ref, &mut changed) {
        eprintln!("error: {message}");
        return ExitCode::FAILURE;
    }

    if changed.is_empty() {
        if args.json {
            println!(
                "{}",
                serde_json::json!({
                    "since": git_ref,
                    "affected_packages": [],
                    "total_packages": 0,
                    "changed_files": 0
                })
            );
        } else {
            println!("No changes since `{git_ref}`; nothing to do.");
        }
        return ExitCode::SUCCESS;
    }

    let paths: Vec<PathBuf> = changed
        .iter()
        .map(|p| {
            let full = repo_root.join(p);
            std::fs::canonicalize(&full).unwrap_or(full)
        })
        .collect();
    let path_refs: Vec<&Path> = paths.iter().map(|p| p.as_path()).collect();

    let package_graph = match if mode == BuildMode::Test {
        project.build_test_graph()
    } else {
        project.build_graph()
    } {
        Ok(graph) => graph,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };

    let owner_ids: Vec<fish_graph::NodeId> = match project.packages_for_paths(&path_refs) {
        Some(owners) => package_graph
            .nodes()
            .iter()
            .filter(|node| owners.iter().any(|owner| &node.payload == owner))
            .map(|node| node.id)
            .collect(),
        None => {
            if !args.common.tui && !args.json {
                println!("Workspace-level file changed; all packages are affected.");
            }
            package_graph.nodes().iter().map(|node| node.id).collect()
        }
    };

    let affected = package_graph.affected_nodes(&owner_ids);
    if affected.is_empty() {
        if args.json {
            println!(
                "{}",
                serde_json::json!({
                    "since": git_ref,
                    "affected_packages": [],
                    "total_packages": package_graph.len(),
                    "changed_files": changed.len()
                })
            );
        } else {
            println!("No packages affected by changes since `{git_ref}`.");
        }
        return ExitCode::SUCCESS;
    }

    let mut affected_names = Vec::new();
    for id in &affected {
        if let Some(package) = package_graph.node(*id)
            && let Some(pkg) = project.package(&package.payload)
        {
            affected_names.push(pkg.name.clone());
        }
    }

    if args.json {
        println!(
            "{}",
            serde_json::json!({
                "since": git_ref,
                "affected_packages": affected_names,
                "total_packages": package_graph.len(),
                "changed_files": changed.len()
            })
        );
        return ExitCode::SUCCESS;
    }

    if !args.common.tui {
        println!(
            "Affected packages ({} of {}):",
            affected.len(),
            package_graph.len()
        );
        for name in &affected_names {
            println!("  - {name}");
        }
        println!();
    }

    let filtered = package_graph.subgraph(&affected);
    build::run_build_mode_with(args.common, mode, Some(filtered))
}

fn git_output(dir: &Path, args: &[&str]) -> Result<String, String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run `git`: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "`git {}` exited with {}: {}",
            args.join(" "),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn collect_git_changes(root: &Path, since: &str, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let diff = git_output(root, &["diff", "--name-only", since])?;
    for line in diff.lines() {
        let line = line.trim();
        if !line.is_empty() {
            out.push(PathBuf::from(line));
        }
    }
    let untracked = git_output(root, &["ls-files", "--others", "--exclude-standard"])?;
    for line in untracked.lines() {
        let line = line.trim();
        if !line.is_empty() {
            out.push(PathBuf::from(line));
        }
    }
    Ok(())
}
