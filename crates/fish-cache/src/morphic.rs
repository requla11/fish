use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DualKeyFingerprint {
    pub exact_key: String,
    pub morphic_key: String,
    pub task_name: String,
    pub normalizations_applied: Vec<String>,
}

const WHITELIST_ENV: &[&str] = &[
    "CGO_ENABLED",
    "CFLAGS",
    "CXXFLAGS",
    "DOTNET_CONFIGURATION",
    "FISH_OFFLINE",
    "FISH_SANDBOX_PROFILE",
    "GOARCH",
    "GOOS",
    "LDFLAGS",
    "NODE_ENV",
    "PYTHONOPTIMIZE",
    "RUSTFLAGS",
    "SWIFT_FLAGS",
];

#[derive(Debug, Clone, Default)]
pub struct MorphicEnvironmentFilter;

impl MorphicEnvironmentFilter {
    pub fn sanitize_env(&self, env: &HashMap<String, String>) -> Vec<(String, String)> {
        let mut filtered: Vec<(String, String)> = env
            .iter()
            .filter(|(k, _)| {
                WHITELIST_ENV.binary_search(&k.as_str()).is_ok() || k.starts_with("FISH_BUILD_")
            })
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        filtered.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        filtered
    }
}

pub struct MorphicPathNormalizer;

impl MorphicPathNormalizer {
    pub fn canonicalize_path(path: &Path, workspace_root: &Path) -> String {
        if let Ok(rel) = path.strip_prefix(workspace_root) {
            format!("$WORKSPACE/{}", rel.to_string_lossy().replace('\\', "/"))
        } else {
            path.to_string_lossy().replace('\\', "/")
        }
    }
}

pub struct MorphicSourceNormalizer;

impl MorphicSourceNormalizer {
    pub fn normalize_source(raw: &str) -> String {
        let mut out = String::with_capacity(raw.len());
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") {
                continue;
            }
            out.push_str(trimmed);
            out.push('\n');
        }
        out
    }
}

pub struct MorphicFingerprintEngine {
    env_filter: MorphicEnvironmentFilter,
}

impl Default for MorphicFingerprintEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl MorphicFingerprintEngine {
    pub fn new() -> Self {
        Self {
            env_filter: MorphicEnvironmentFilter,
        }
    }

    pub fn compute_dual_key(
        &self,
        task_name: &str,
        workspace_root: &Path,
        input_files: &[(PathBuf, &str)],
        argv: &[String],
        raw_env: &HashMap<String, String>,
    ) -> DualKeyFingerprint {
        let mut exact_hasher = blake3::Hasher::new();
        exact_hasher.update(task_name.as_bytes());
        exact_hasher.update(workspace_root.to_string_lossy().as_bytes());
        for arg in argv {
            exact_hasher.update(arg.as_bytes());
        }
        let mut sorted_raw_env: Vec<(&String, &String)> = raw_env.iter().collect();
        sorted_raw_env.sort_unstable_by(|a, b| a.0.cmp(b.0));
        for (k, v) in sorted_raw_env {
            exact_hasher.update(k.as_bytes());
            exact_hasher.update(v.as_bytes());
        }
        for (path, content) in input_files {
            exact_hasher.update(path.to_string_lossy().as_bytes());
            exact_hasher.update(content.as_bytes());
        }
        let exact_key = exact_hasher.finalize().to_hex().to_string();

        let mut normalizations = Vec::new();
        let mut morphic_hasher = blake3::Hasher::new();
        morphic_hasher.update(task_name.as_bytes());
        for arg in argv {
            morphic_hasher.update(arg.as_bytes());
        }

        let filtered_env = self.env_filter.sanitize_env(raw_env);
        if filtered_env.len() < raw_env.len() {
            normalizations.push("environment_entropy_pruned".to_string());
        }
        for (k, v) in &filtered_env {
            morphic_hasher.update(k.as_bytes());
            morphic_hasher.update(v.as_bytes());
        }

        let mut sorted_files: Vec<&(PathBuf, &str)> = input_files.iter().collect();
        sorted_files.sort_unstable_by(|a, b| a.0.cmp(&b.0));
        for (path, content) in sorted_files {
            let canon_path = MorphicPathNormalizer::canonicalize_path(path, workspace_root);
            if canon_path.starts_with("$WORKSPACE") {
                normalizations.push(format!("path_relativized:{canon_path}"));
            }
            morphic_hasher.update(canon_path.as_bytes());
            let normalized_src = MorphicSourceNormalizer::normalize_source(content);
            if normalized_src.len() != content.len() {
                normalizations.push("source_whitespace_normalized".to_string());
            }
            morphic_hasher.update(normalized_src.as_bytes());
        }

        let morphic_key = morphic_hasher.finalize().to_hex().to_string();
        DualKeyFingerprint {
            exact_key,
            morphic_key,
            task_name: task_name.to_string(),
            normalizations_applied: normalizations,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MorphicLookupResult {
    ExactHit {
        artifact_digest: String,
    },
    MorphicHit {
        artifact_digest: String,
        confidence: u32,
        transformations: Vec<String>,
    },
    Miss,
}

#[derive(Debug, Clone, Default)]
pub struct MorphicCacheCatalog {
    exact_index: HashMap<String, String>,
    morphic_index: HashMap<String, String>,
}

impl MorphicCacheCatalog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, fingerprint: &DualKeyFingerprint, artifact_digest: &str) {
        self.exact_index
            .insert(fingerprint.exact_key.clone(), artifact_digest.to_string());
        self.morphic_index
            .insert(fingerprint.morphic_key.clone(), artifact_digest.to_string());
    }

    pub fn query(&self, fingerprint: &DualKeyFingerprint) -> MorphicLookupResult {
        if let Some(digest) = self.exact_index.get(&fingerprint.exact_key) {
            return MorphicLookupResult::ExactHit {
                artifact_digest: digest.clone(),
            };
        }

        if let Some(digest) = self.morphic_index.get(&fingerprint.morphic_key) {
            return MorphicLookupResult::MorphicHit {
                artifact_digest: digest.clone(),
                confidence: 98,
                transformations: fingerprint.normalizations_applied.clone(),
            };
        }

        MorphicLookupResult::Miss
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_and_env_variance_yields_identical_morphic_key() {
        let engine = MorphicFingerprintEngine::new();
        let argv = vec!["cargo".to_string(), "build".to_string()];

        let mut env_local = HashMap::new();
        env_local.insert("USER".to_string(), "alice".to_string());
        env_local.insert("PWD".to_string(), "/home/alice/repo".to_string());
        env_local.insert("RUSTFLAGS".to_string(), "-C opt-level=3".to_string());

        let mut env_ci = HashMap::new();
        env_ci.insert("USER".to_string(), "runner".to_string());
        env_ci.insert("PWD".to_string(), "/runner/work/1/repo".to_string());
        env_ci.insert("RUSTFLAGS".to_string(), "-C opt-level=3".to_string());

        let root_local = Path::new("/home/alice/repo");
        let root_ci = Path::new("/runner/work/1/repo");

        let file_local = (
            root_local.join("src/lib.rs"),
            "fn hello() -> i32 {\n    42\n}\n",
        );
        let file_ci = (
            root_ci.join("src/lib.rs"),
            "fn hello() -> i32 {\n    42\n}\n",
        );

        let fp_local =
            engine.compute_dual_key("compile_lib", root_local, &[file_local], &argv, &env_local);
        let fp_ci = engine.compute_dual_key("compile_lib", root_ci, &[file_ci], &argv, &env_ci);

        assert_ne!(fp_local.exact_key, fp_ci.exact_key);
        assert_eq!(fp_local.morphic_key, fp_ci.morphic_key);
    }

    #[test]
    fn test_morphic_catalog_falls_back_to_morphic_hit() {
        let engine = MorphicFingerprintEngine::new();
        let mut catalog = MorphicCacheCatalog::new();
        let argv = vec!["gcc".to_string(), "-O2".to_string()];

        let root_workstation = Path::new("/Users/dev/fish");
        let root_ci = Path::new("/home/ci/agent/fish");

        let mut env1 = HashMap::new();
        env1.insert("CFLAGS".to_string(), "-Wall".to_string());
        env1.insert("LOGNAME".to_string(), "dev".to_string());

        let mut env2 = HashMap::new();
        env2.insert("CFLAGS".to_string(), "-Wall".to_string());
        env2.insert("LOGNAME".to_string(), "ci-agent-04".to_string());

        let f1 = (root_workstation.join("main.c"), "int main() { return 0; }");
        let f2 = (root_ci.join("main.c"), "int main() { return 0; }");

        let fp1 = engine.compute_dual_key("build_c", root_workstation, &[f1], &argv, &env1);
        let fp2 = engine.compute_dual_key("build_c", root_ci, &[f2], &argv, &env2);

        catalog.insert(&fp1, "blake3_digest_abc123");

        let res = catalog.query(&fp2);
        match res {
            MorphicLookupResult::MorphicHit {
                artifact_digest,
                confidence,
                ..
            } => {
                assert_eq!(artifact_digest, "blake3_digest_abc123");
                assert_eq!(confidence, 98);
            }
            _ => panic!("Expected MorphicHit"),
        }
    }
}
