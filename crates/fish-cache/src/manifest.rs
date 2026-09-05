use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const DEFAULT_IGNORED_DIRS: &[&str] = &[
    ".git",
    ".fish",
    "target",
    "build",
    "dist",
    "node_modules",
    "vendor",
    ".dart_tool",
    "__pycache__",
    ".venv",
    "venv",
    "bin",
    "obj",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileDigest {
    pub path: String,
    pub hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskManifest {
    pub key: String,
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,
    pub inputs: Vec<FileDigest>,
    pub upstream_deps: BTreeMap<String, String>,
    pub total_fingerprint: String,
    pub stored_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ManifestVerdict {
    ColdMiss,
    ExactMatch,
    Drifted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    pub old_hash: String,
    pub new_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvDiff {
    pub key: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DepDiff {
    pub label: String,
    pub old_fingerprint: Option<String>,
    pub new_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestDiff {
    pub target: String,
    pub verdict: ManifestVerdict,
    pub modified_files: Vec<FileDiff>,
    pub added_files: Vec<String>,
    pub removed_files: Vec<String>,
    pub changed_envs: Vec<EnvDiff>,
    pub changed_args: Option<(Vec<String>, Vec<String>)>,
    pub changed_deps: Vec<DepDiff>,
    pub old_fingerprint: Option<String>,
    pub new_fingerprint: Option<String>,
}

pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

pub fn hash_single_file(path: &Path) -> Option<(String, u64)> {
    let mut file = fs::File::open(path).ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file() {
        return None;
    }
    let size = meta.len();
    if crate::lockfile_hash::LockfileHasher::detect_kind(path)
        != crate::lockfile_hash::LockfileKind::Generic
        && size <= 16 * 1024 * 1024
    {
        let mut content = Vec::with_capacity(size as usize);
        file.read_to_end(&mut content).ok()?;
        let hash = crate::lockfile_hash::LockfileHasher::compute_canonical_hash(path, &content);
        return Some((hash, size));
    }
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    Some((hasher.finalize().to_hex().to_string(), size))
}

fn collect_directory_files(
    current: &Path,
    base: &Path,
    out: &mut Vec<FileDigest>,
) -> std::io::Result<()> {
    if !current.exists() || !current.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(current)?;
    for entry in entries.flatten() {
        let path = entry.path();
        let file_name = entry.file_name();
        let name_str = file_name.to_string_lossy();
        if let Ok(ft) = entry.file_type() {
            if ft.is_dir() {
                if DEFAULT_IGNORED_DIRS.contains(&name_str.as_ref()) {
                    continue;
                }
                collect_directory_files(&path, base, out)?;
            } else if ft.is_file()
                && let Some((hash, size)) = hash_single_file(&path)
            {
                let rel = path.strip_prefix(base).unwrap_or(&path);
                out.push(FileDigest {
                    path: normalize_path(rel),
                    hash,
                    size,
                });
            }
        }
    }
    Ok(())
}

impl TaskManifest {
    pub fn from_task(
        task: &fish_executor::Task,
        upstream_fingerprints: &BTreeMap<String, String>,
    ) -> Self {
        let cwd = task.spec.cwd.clone().unwrap_or_else(|| PathBuf::from("."));
        let mut file_digests: Vec<FileDigest> = Vec::new();

        if !task.inputs.is_empty() {
            for input_path in &task.inputs {
                let absolute = if input_path.is_absolute() {
                    input_path.clone()
                } else {
                    cwd.join(input_path)
                };
                if absolute.is_file() {
                    if let Some((hash, size)) = hash_single_file(&absolute) {
                        let rel = absolute.strip_prefix(&cwd).unwrap_or(&absolute);
                        file_digests.push(FileDigest {
                            path: normalize_path(rel),
                            hash,
                            size,
                        });
                    }
                } else if absolute.is_dir() {
                    let _ = collect_directory_files(&absolute, &cwd, &mut file_digests);
                }
            }
        } else if cwd.is_dir() {
            let _ = collect_directory_files(&cwd, &cwd, &mut file_digests);
        }

        file_digests.sort_by(|a, b| a.path.cmp(&b.path));
        file_digests.dedup_by(|a, b| a.path == b.path);

        let mut env_map = task.spec.env.clone();
        let critical_envs = [
            "RUSTFLAGS",
            "CC",
            "CXX",
            "CFLAGS",
            "CXXFLAGS",
            "LDFLAGS",
            "GOOS",
            "GOARCH",
            "CGO_ENABLED",
            "NODE_ENV",
            "PYTHONPATH",
        ];
        for var in critical_envs {
            if !env_map.contains_key(var)
                && let Ok(val) = std::env::var(var)
            {
                env_map.insert(var.to_string(), val);
            }
        }

        let key = task
            .cache
            .as_ref()
            .map(|c| c.key.clone())
            .unwrap_or_else(|| task.label.clone());
        let total_fingerprint = task
            .cache
            .as_ref()
            .map(|c| c.fingerprint.clone())
            .unwrap_or_default();

        let stored_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Self {
            key,
            label: task.label.clone(),
            command: task.spec.program.clone(),
            args: task.spec.args.clone(),
            env: env_map,
            inputs: file_digests,
            upstream_deps: upstream_fingerprints.clone(),
            total_fingerprint,
            stored_at,
        }
    }

    pub fn diff(&self, current: &TaskManifest) -> ManifestDiff {
        let mut old_inputs: BTreeMap<String, &FileDigest> = BTreeMap::new();
        for f in &self.inputs {
            old_inputs.insert(f.path.clone(), f);
        }

        let mut new_inputs: BTreeMap<String, &FileDigest> = BTreeMap::new();
        for f in &current.inputs {
            new_inputs.insert(f.path.clone(), f);
        }

        let mut modified_files = Vec::new();
        let mut removed_files = Vec::new();
        for (path, old_dig) in &old_inputs {
            match new_inputs.get(path) {
                Some(new_dig) => {
                    if old_dig.hash != new_dig.hash {
                        modified_files.push(FileDiff {
                            path: path.clone(),
                            old_hash: old_dig.hash.clone(),
                            new_hash: new_dig.hash.clone(),
                        });
                    }
                }
                None => {
                    removed_files.push(path.clone());
                }
            }
        }

        let mut added_files = Vec::new();
        for path in new_inputs.keys() {
            if !old_inputs.contains_key(path) {
                added_files.push(path.clone());
            }
        }

        let mut all_env_keys: BTreeSet<String> = BTreeSet::new();
        all_env_keys.extend(self.env.keys().cloned());
        all_env_keys.extend(current.env.keys().cloned());

        let mut changed_envs = Vec::new();
        for k in all_env_keys {
            let old_val = self.env.get(&k).cloned();
            let new_val = current.env.get(&k).cloned();
            if old_val != new_val {
                changed_envs.push(EnvDiff {
                    key: k,
                    old_value: old_val,
                    new_value: new_val,
                });
            }
        }

        let changed_args = if self.args != current.args {
            Some((self.args.clone(), current.args.clone()))
        } else {
            None
        };

        let mut all_deps: BTreeSet<String> = BTreeSet::new();
        all_deps.extend(self.upstream_deps.keys().cloned());
        all_deps.extend(current.upstream_deps.keys().cloned());

        let mut changed_deps = Vec::new();
        for d in all_deps {
            let old_fp = self.upstream_deps.get(&d).cloned();
            let new_fp = current.upstream_deps.get(&d).cloned();
            if old_fp != new_fp {
                changed_deps.push(DepDiff {
                    label: d,
                    old_fingerprint: old_fp,
                    new_fingerprint: new_fp,
                });
            }
        }

        let is_exact = modified_files.is_empty()
            && added_files.is_empty()
            && removed_files.is_empty()
            && changed_envs.is_empty()
            && changed_args.is_none()
            && changed_deps.is_empty()
            && self.command == current.command
            && self.total_fingerprint == current.total_fingerprint;

        let verdict = if is_exact {
            ManifestVerdict::ExactMatch
        } else {
            ManifestVerdict::Drifted
        };

        ManifestDiff {
            target: current.label.clone(),
            verdict,
            modified_files,
            added_files,
            removed_files,
            changed_envs,
            changed_args,
            changed_deps,
            old_fingerprint: Some(self.total_fingerprint.clone()),
            new_fingerprint: Some(current.total_fingerprint.clone()),
        }
    }

    pub fn diff_against_working_tree(&self, cwd: &Path) -> ManifestDiff {
        let mut modified_files = Vec::new();
        let mut removed_files = Vec::new();

        for old_file in &self.inputs {
            let abs_path = cwd.join(&old_file.path);
            if !abs_path.exists() {
                removed_files.push(old_file.path.clone());
            } else if let Some((current_hash, _)) = hash_single_file(&abs_path) {
                if current_hash != old_file.hash {
                    modified_files.push(FileDiff {
                        path: old_file.path.clone(),
                        old_hash: old_file.hash.clone(),
                        new_hash: current_hash,
                    });
                }
            } else {
                removed_files.push(old_file.path.clone());
            }
        }

        let mut current_all_files = Vec::new();
        if cwd.is_dir() {
            let _ = collect_directory_files(cwd, cwd, &mut current_all_files);
        }
        let old_path_set: BTreeSet<String> = self.inputs.iter().map(|f| f.path.clone()).collect();
        let mut added_files = Vec::new();
        for cur in current_all_files {
            if !old_path_set.contains(&cur.path) {
                added_files.push(cur.path);
            }
        }

        let mut changed_envs = Vec::new();
        for (k, old_val) in &self.env {
            let cur_val = std::env::var(k).ok();
            if cur_val.as_ref() != Some(old_val) {
                changed_envs.push(EnvDiff {
                    key: k.clone(),
                    old_value: Some(old_val.clone()),
                    new_value: cur_val,
                });
            }
        }

        let is_exact = modified_files.is_empty()
            && added_files.is_empty()
            && removed_files.is_empty()
            && changed_envs.is_empty();

        let verdict = if is_exact {
            ManifestVerdict::ExactMatch
        } else {
            ManifestVerdict::Drifted
        };

        ManifestDiff {
            target: self.label.clone(),
            verdict,
            modified_files,
            added_files,
            removed_files,
            changed_envs,
            changed_args: None,
            changed_deps: Vec::new(),
            old_fingerprint: Some(self.total_fingerprint.clone()),
            new_fingerprint: None,
        }
    }
}

impl ManifestDiff {
    pub fn cold_miss(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
            verdict: ManifestVerdict::ColdMiss,
            modified_files: Vec::new(),
            added_files: Vec::new(),
            removed_files: Vec::new(),
            changed_envs: Vec::new(),
            changed_args: None,
            changed_deps: Vec::new(),
            old_fingerprint: None,
            new_fingerprint: None,
        }
    }

    pub fn format_explanation(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "=== Rebuild explanation for `{}` ===\n",
            self.target
        ));
        match self.verdict {
            ManifestVerdict::ColdMiss => {
                out.push_str("Verdict: Cold cache miss\n");
                out.push_str("No cached fingerprint or manifest found for this target.\n");
                out.push_str(
                    "The upcoming build execution will be executed and populate the cache.\n",
                );
                return out;
            }
            ManifestVerdict::ExactMatch => {
                out.push_str("Verdict: Cache hit (all inputs match previous build)\n");
                if let Some(fp) = &self.old_fingerprint {
                    let short_fp = &fp[..fp.len().min(16)];
                    out.push_str(&format!("Cached fingerprint: {}...\n", short_fp));
                }
                out.push_str(
                    "No rebuild necessary; outputs can be securely restored from CAS cache.\n",
                );
                return out;
            }
            ManifestVerdict::Drifted => {
                out.push_str("Verdict: Fingerprint drift detected (rebuild required)\n");
                if let (Some(old_fp), Some(new_fp)) = (&self.old_fingerprint, &self.new_fingerprint)
                {
                    let old_short = &old_fp[..old_fp.len().min(12)];
                    let new_short = &new_fp[..new_fp.len().min(12)];
                    out.push_str(&format!(
                        "Fingerprint transition: {}... -> {}...\n",
                        old_short, new_short
                    ));
                } else if let Some(old_fp) = &self.old_fingerprint {
                    let old_short = &old_fp[..old_fp.len().min(12)];
                    out.push_str(&format!("Recorded fingerprint: {}...\n", old_short));
                }
            }
        }

        if !self.modified_files.is_empty() {
            out.push_str(&format!(
                "\nModified files ({}):\n",
                self.modified_files.len()
            ));
            for f in &self.modified_files {
                let old_short = &f.old_hash[..f.old_hash.len().min(8)];
                let new_short = &f.new_hash[..f.new_hash.len().min(8)];
                out.push_str(&format!(
                    "  ~ {} (blake3: {}... -> {}...)\n",
                    f.path, old_short, new_short
                ));
            }
        }

        if !self.added_files.is_empty() {
            out.push_str(&format!("\nAdded files ({}):\n", self.added_files.len()));
            for f in &self.added_files {
                out.push_str(&format!("  + {}\n", f));
            }
        }

        if !self.removed_files.is_empty() {
            out.push_str(&format!(
                "\nRemoved files ({}):\n",
                self.removed_files.len()
            ));
            for f in &self.removed_files {
                out.push_str(&format!("  - {}\n", f));
            }
        }

        if !self.changed_envs.is_empty() {
            out.push_str(&format!(
                "\nEnvironment variables changed ({}):\n",
                self.changed_envs.len()
            ));
            for e in &self.changed_envs {
                let old_str = e.old_value.as_deref().unwrap_or("<unset>");
                let new_str = e.new_value.as_deref().unwrap_or("<unset>");
                out.push_str(&format!(
                    "  ~ {}: \"{}\" -> \"{}\"\n",
                    e.key, old_str, new_str
                ));
            }
        }

        if let Some((old_args, new_args)) = &self.changed_args {
            out.push_str("\nCommand arguments changed:\n");
            out.push_str(&format!("  Old: {:?}\n", old_args));
            out.push_str(&format!("  New: {:?}\n", new_args));
        }

        if !self.changed_deps.is_empty() {
            out.push_str(&format!(
                "\nUpstream dependencies changed ({}):\n",
                self.changed_deps.len()
            ));
            for d in &self.changed_deps {
                let old_str = d.old_fingerprint.as_deref().unwrap_or("<none>");
                let new_str = d.new_fingerprint.as_deref().unwrap_or("<none>");
                let old_short = &old_str[..old_str.len().min(8)];
                let new_short = &new_str[..new_str.len().min(8)];
                out.push_str(&format!(
                    "  ~ {}: {}... -> {}...\n",
                    d.label, old_short, new_short
                ));
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fish_executor::CommandSpec;

    #[test]
    fn test_task_manifest_diff_exact_match() {
        let manifest = TaskManifest {
            key: "task-a".to_string(),
            label: "compile-a".to_string(),
            command: "rustc".to_string(),
            args: vec!["main.rs".to_string()],
            env: BTreeMap::new(),
            inputs: vec![FileDigest {
                path: "main.rs".to_string(),
                hash: "11223344".to_string(),
                size: 100,
            }],
            upstream_deps: BTreeMap::new(),
            total_fingerprint: "fp123".to_string(),
            stored_at: 1000,
        };

        let diff = manifest.diff(&manifest);
        assert_eq!(diff.verdict, ManifestVerdict::ExactMatch);
        assert!(diff.modified_files.is_empty());
        assert!(diff.added_files.is_empty());
        assert!(diff.removed_files.is_empty());
    }

    #[test]
    fn test_task_manifest_diff_modified_file() {
        let manifest_old = TaskManifest {
            key: "task-a".to_string(),
            label: "compile-a".to_string(),
            command: "rustc".to_string(),
            args: vec!["main.rs".to_string()],
            env: BTreeMap::new(),
            inputs: vec![FileDigest {
                path: "main.rs".to_string(),
                hash: "11223344".to_string(),
                size: 100,
            }],
            upstream_deps: BTreeMap::new(),
            total_fingerprint: "fp123".to_string(),
            stored_at: 1000,
        };

        let mut manifest_new = manifest_old.clone();
        manifest_new.inputs[0].hash = "55667788".to_string();
        manifest_new.total_fingerprint = "fp456".to_string();

        let diff = manifest_old.diff(&manifest_new);
        assert_eq!(diff.verdict, ManifestVerdict::Drifted);
        assert_eq!(diff.modified_files.len(), 1);
        assert_eq!(diff.modified_files[0].path, "main.rs");
        assert_eq!(diff.modified_files[0].old_hash, "11223344");
        assert_eq!(diff.modified_files[0].new_hash, "55667788");

        let formatted = diff.format_explanation();
        assert!(formatted.contains("Fingerprint drift detected"));
        assert!(formatted.contains("Modified files (1):"));
        assert!(formatted.contains("main.rs"));
    }

    #[test]
    fn test_task_manifest_diff_env_and_args() {
        let mut env_old = BTreeMap::new();
        env_old.insert("RUSTFLAGS".to_string(), "-O".to_string());

        let manifest_old = TaskManifest {
            key: "task-b".to_string(),
            label: "compile-b".to_string(),
            command: "cargo".to_string(),
            args: vec!["build".to_string()],
            env: env_old,
            inputs: Vec::new(),
            upstream_deps: BTreeMap::new(),
            total_fingerprint: "fp_old".to_string(),
            stored_at: 1000,
        };

        let mut env_new = BTreeMap::new();
        env_new.insert("RUSTFLAGS".to_string(), "-O -C debuginfo=2".to_string());

        let manifest_new = TaskManifest {
            key: "task-b".to_string(),
            label: "compile-b".to_string(),
            command: "cargo".to_string(),
            args: vec!["build".to_string(), "--release".to_string()],
            env: env_new,
            inputs: Vec::new(),
            upstream_deps: BTreeMap::new(),
            total_fingerprint: "fp_new".to_string(),
            stored_at: 2000,
        };

        let diff = manifest_old.diff(&manifest_new);
        assert_eq!(diff.verdict, ManifestVerdict::Drifted);
        assert_eq!(diff.changed_envs.len(), 1);
        assert_eq!(diff.changed_envs[0].key, "RUSTFLAGS");
        assert!(diff.changed_args.is_some());

        let explanation = diff.format_explanation();
        assert!(explanation.contains("Environment variables changed (1):"));
        assert!(explanation.contains("Command arguments changed:"));
    }

    #[test]
    fn test_from_task_with_inputs() {
        let temp = tempfile::tempdir().unwrap();
        let file_path = temp.path().join("source.txt");
        fs::write(&file_path, b"hello world").unwrap();

        let mut spec = CommandSpec::new("echo");
        spec.cwd = Some(temp.path().to_path_buf());
        let task = fish_executor::Task::new("echo-task", "echo desc", spec)
            .with_inputs(vec![PathBuf::from("source.txt")])
            .with_cache(fish_executor::CacheEntry {
                key: "key-1".to_string(),
                fingerprint: "fp-1".to_string(),
            });

        let manifest = TaskManifest::from_task(&task, &BTreeMap::new());
        assert_eq!(manifest.key, "key-1");
        assert_eq!(manifest.label, "echo-task");
        assert_eq!(manifest.inputs.len(), 1);
        assert_eq!(manifest.inputs[0].path, "source.txt");
        assert_eq!(manifest.inputs[0].size, 11);
    }
}
