//! Toolchain lock file (`fish.lock`) for reproducible builds.
//!
//! Serializes the resolved [`ToolchainRegistry`] into a versioned lock file
//! so CI runners can recreate the exact toolchain environment. Every entry
//! is pinned by kind + version + optional checksum; a `lock_version` field
//! enables future format migrations.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::toolchain::{ToolchainKind, ToolchainRegistry};

pub const LOCK_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolchainLock {
    pub lock_version: u32,
    pub entries: Vec<ToolchainLockEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolchainLockEntry {
    /// Serde tag matching `ToolchainKind` (e.g. `"Rust"`, `"Node"`).
    pub kind: ToolchainKind,
    pub version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    pub hermetic: bool,
}

impl ToolchainLock {
    /// Create a lock from a resolved registry.
    ///
    /// Entries are sorted by kind name for deterministic output.
    pub fn from_registry(registry: &ToolchainRegistry) -> Self {
        let mut entries: Vec<ToolchainLockEntry> = registry
            .all()
            .into_iter()
            .map(|spec| ToolchainLockEntry {
                kind: spec.kind.clone(),
                version: spec.version.clone(),
                checksum: spec.checksum.clone(),
                hermetic: spec.is_hermetic,
            })
            .collect();
        entries.sort_by(|a, b| format!("{:?}", a.kind).cmp(&format!("{:?}", b.kind)));
        Self {
            lock_version: LOCK_VERSION,
            entries,
        }
    }

    /// Load from a `fish.lock` TOML file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let lock: Self = toml::from_str(&content)
            .map_err(|e| format!("invalid lock file {}: {e}", path.display()))?;
        if lock.lock_version != LOCK_VERSION {
            return Err(format!(
                "unsupported lock_version {}; expected {LOCK_VERSION}",
                lock.lock_version
            ));
        }
        Ok(lock)
    }

    /// Write to a `fish.lock` TOML file.
    pub fn save(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        let content =
            toml::to_string_pretty(self).map_err(|e| format!("serialization failed: {e}"))?;
        fs::write(path, content).map_err(|e| format!("cannot write {}: {e}", path.display()))
    }

    /// Check whether the registry matches this lock (kind + version pairs).
    ///
    /// Returns a list of mismatches; empty means fully in sync.
    pub fn verify_against(&self, registry: &ToolchainRegistry) -> Vec<String> {
        let mut mismatches = Vec::new();
        for entry in &self.entries {
            match registry.get(&entry.kind) {
                Some(spec) => {
                    if spec.version != entry.version {
                        mismatches.push(format!(
                            "{:?}: locked `{}`, found `{}`",
                            entry.kind, entry.version, spec.version
                        ));
                    }
                    if entry.hermetic && !spec.is_hermetic {
                        mismatches.push(format!("{:?}: expected hermetic", entry.kind));
                    }
                }
                None => {
                    mismatches.push(format!("{:?}: missing from registry", entry.kind));
                }
            }
        }
        mismatches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolchain::{ToolchainKind, ToolchainSpec};
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn spec(kind: ToolchainKind, version: &str) -> ToolchainSpec {
        ToolchainSpec {
            kind,
            version: version.to_string(),
            path: std::path::PathBuf::from("/fake"),
            envs: HashMap::new(),
            checksum: None,
            is_hermetic: false,
        }
    }

    #[test]
    fn test_lock_roundtrip_through_disk() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("fish.lock");

        let mut registry = ToolchainRegistry::default();
        registry.register(spec(ToolchainKind::Rust, "1.88.0"));
        registry.register(spec(ToolchainKind::Node, "22.0.0"));

        let lock = ToolchainLock::from_registry(&registry);
        lock.save(&path).unwrap();

        let loaded = ToolchainLock::load(&path).unwrap();
        assert_eq!(loaded.lock_version, LOCK_VERSION);
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.verify_against(&registry).len(), 0);
    }

    #[test]
    fn test_verify_detects_version_mismatch() {
        let mut locked_registry = ToolchainRegistry::default();
        locked_registry.register(spec(ToolchainKind::Rust, "1.88.0"));
        let lock = ToolchainLock::from_registry(&locked_registry);

        let mut actual = ToolchainRegistry::default();
        actual.register(spec(ToolchainKind::Rust, "1.90.0"));

        let mismatches = lock.verify_against(&actual);
        assert_eq!(mismatches.len(), 1);
        assert!(mismatches[0].contains("1.88.0"), "got: {}", mismatches[0]);
    }

    #[test]
    fn test_verify_detects_missing_toolchain() {
        let mut locked_registry = ToolchainRegistry::default();
        locked_registry.register(spec(ToolchainKind::Go, "1.22.0"));
        locked_registry.register(spec(ToolchainKind::Node, "22.0.0"));
        let lock = ToolchainLock::from_registry(&locked_registry);

        let empty = ToolchainRegistry::default();
        assert_eq!(lock.verify_against(&empty).len(), 2);
    }
}
