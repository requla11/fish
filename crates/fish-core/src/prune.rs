use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::project::Project;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PruneResult {
    pub target: String,
    pub packages_included: Vec<String>,
    pub files_copied: usize,
    pub bytes_copied: u64,
    pub out_dir: PathBuf,
}

fn copy_file_tracked(
    src: &Path,
    dst: &Path,
    files_copied: &mut usize,
    bytes_copied: &mut u64,
) -> std::io::Result<()> {
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = fs::copy(src, dst)?;
    *files_copied += 1;
    *bytes_copied += bytes;
    Ok(())
}

fn should_skip_entry(name: &str) -> bool {
    matches!(
        name,
        "target" | "node_modules" | ".git" | "dist" | ".turbo" | ".fish" | "build" | ".cache"
    )
}

fn copy_dir_all(
    src: &Path,
    dst: &Path,
    files_copied: &mut usize,
    bytes_copied: &mut u64,
) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        let name = file_name.to_string_lossy();
        if should_skip_entry(&name) {
            continue;
        }
        let file_type = entry.file_type()?;
        let dest_path = dst.join(&file_name);
        if file_type.is_dir() {
            copy_dir_all(&entry.path(), &dest_path, files_copied, bytes_copied)?;
        } else if file_type.is_file() {
            copy_file_tracked(&entry.path(), &dest_path, files_copied, bytes_copied)?;
        }
    }
    Ok(())
}

pub fn prune_workspace(
    project: &Project,
    target_name: &str,
    out_dir: &Path,
    _docker_mode: bool,
) -> Result<PruneResult, String> {
    let metadata = project.metadata();
    let root_path = metadata.workspace_root.as_std_path();

    let _target_pkg = metadata
        .packages
        .iter()
        .find(|p| p.name == target_name)
        .ok_or_else(|| format!("target package '{target_name}' not found in workspace"))?;

    let workspace_pkg_names: HashSet<&str> = metadata
        .workspace_members
        .iter()
        .filter_map(|id| project.package(id))
        .map(|p| p.name.as_str())
        .collect();

    let mut included_names: HashSet<String> = HashSet::new();
    let mut queue: VecDeque<String> = VecDeque::new();

    included_names.insert(target_name.to_string());
    queue.push_back(target_name.to_string());

    if let Some(resolve) = &metadata.resolve {
        let node_map: std::collections::HashMap<&str, &cargo_metadata::Node> = resolve
            .nodes
            .iter()
            .filter_map(|node| project.package(&node.id).map(|p| (p.name.as_str(), node)))
            .collect();

        while let Some(current) = queue.pop_front() {
            if let Some(node) = node_map.get(current.as_str()) {
                for dep in &node.deps {
                    if let Some(dep_pkg) = project.package(&dep.pkg) {
                        let dep_name = dep_pkg.name.as_str();
                        if workspace_pkg_names.contains(dep_name)
                            && included_names.insert(dep_name.to_string())
                        {
                            queue.push_back(dep_name.to_string());
                        }
                    }
                }
            }
        }
    }

    let json_dir = out_dir.join("json");
    let full_dir = out_dir.join("full");
    fs::create_dir_all(&json_dir).map_err(|e| e.to_string())?;
    fs::create_dir_all(&full_dir).map_err(|e| e.to_string())?;

    let mut files_copied = 0usize;
    let mut bytes_copied = 0u64;

    let root_manifest_files = [
        "Cargo.toml",
        "Cargo.lock",
        "fish.toml",
        "package.json",
        "pnpm-lock.yaml",
        "yarn.lock",
        "bun.lockb",
        "go.mod",
        "go.sum",
        "deno.json",
        ".npmrc",
    ];

    for file_name in root_manifest_files {
        let src_file = root_path.join(file_name);
        if src_file.is_file() {
            copy_file_tracked(
                &src_file,
                &json_dir.join(file_name),
                &mut files_copied,
                &mut bytes_copied,
            )
            .map_err(|e| e.to_string())?;
            copy_file_tracked(
                &src_file,
                &full_dir.join(file_name),
                &mut files_copied,
                &mut bytes_copied,
            )
            .map_err(|e| e.to_string())?;
        }
    }

    let mut sorted_included: Vec<String> = included_names.into_iter().collect();
    sorted_included.sort();

    for pkg_name in &sorted_included {
        let pkg = metadata
            .packages
            .iter()
            .find(|p| p.name == *pkg_name)
            .unwrap();

        let pkg_manifest = pkg.manifest_path.as_std_path();
        let Some(pkg_dir) = pkg_manifest.parent() else {
            continue;
        };

        let Ok(rel_path) = pkg_dir.strip_prefix(root_path) else {
            continue;
        };

        let target_json_pkg_dir = json_dir.join(rel_path);
        let target_full_pkg_dir = full_dir.join(rel_path);

        if pkg_manifest.is_file() {
            let manifest_filename = pkg_manifest.file_name().unwrap_or_default();
            copy_file_tracked(
                pkg_manifest,
                &target_json_pkg_dir.join(manifest_filename),
                &mut files_copied,
                &mut bytes_copied,
            )
            .map_err(|e| e.to_string())?;
        }

        let package_json = pkg_dir.join("package.json");
        if package_json.is_file() {
            copy_file_tracked(
                &package_json,
                &target_json_pkg_dir.join("package.json"),
                &mut files_copied,
                &mut bytes_copied,
            )
            .map_err(|e| e.to_string())?;
        }

        copy_dir_all(
            pkg_dir,
            &target_full_pkg_dir,
            &mut files_copied,
            &mut bytes_copied,
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(PruneResult {
        target: target_name.to_string(),
        packages_included: sorted_included,
        files_copied,
        bytes_copied,
        out_dir: out_dir.to_path_buf(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_entry_rules() {
        assert!(should_skip_entry("target"));
        assert!(should_skip_entry("node_modules"));
        assert!(should_skip_entry(".git"));
        assert!(should_skip_entry(".turbo"));
        assert!(!should_skip_entry("src"));
        assert!(!should_skip_entry("Cargo.toml"));
    }
}
