//! Environment drift detector.
//!
//! Hashes the [`EnvironmentFingerprint`] after every successful build and
//! compares it against the previous run's hash. When the environment changes
//! (compiler upgrade, OS patch, toolchain swap) the detector warns so stale
//! cache entries can be invalidated proactively.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::environment::EnvironmentFingerprint;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftRecord {
    pub fingerprint_hash: String,
    pub timestamp_unix_secs: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DriftVerdict {
    /// No previous record; nothing to compare against.
    FirstRun,
    /// Environment matches the previous build.
    Stable,
    /// Environment changed since the last build.
    Drifted {
        previous_hash: String,
        current_hash: String,
    },
}

pub struct DriftDetector {
    path: PathBuf,
}

impl DriftDetector {
    pub fn new(project_root: &Path) -> Self {
        Self {
            path: project_root
                .join(".fish")
                .join("metrics")
                .join("env_drift.json"),
        }
    }

    /// Record the current environment hash and compare with the last one.
    pub fn check_and_record(
        &self,
        fingerprint: &EnvironmentFingerprint,
    ) -> std::io::Result<DriftVerdict> {
        let current_hash = compute_fingerprint_hash(fingerprint);
        let previous = self.load_previous()?;

        let verdict = match &previous {
            None => DriftVerdict::FirstRun,
            Some(prev) if prev.fingerprint_hash == current_hash => DriftVerdict::Stable,
            Some(prev) => DriftVerdict::Drifted {
                previous_hash: prev.fingerprint_hash.clone(),
                current_hash: current_hash.clone(),
            },
        };

        // Save the new record.
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let record = DriftRecord {
            fingerprint_hash: current_hash,
            timestamp_unix_secs: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };
        let json = serde_json::to_string_pretty(&record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        fs::write(&self.path, json)?;

        Ok(verdict)
    }

    fn load_previous(&self) -> std::io::Result<Option<DriftRecord>> {
        if !self.path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&self.path)?;
        serde_json::from_str(&content)
            .map(Some)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

/// Deterministic hash over the fingerprint's meaningful fields.
fn compute_fingerprint_hash(fp: &EnvironmentFingerprint) -> String {
    let mut hasher = blake3::Hasher::new();
    hasher.update(fp.os.as_bytes());
    hasher.update(fp.os_version.as_bytes());
    hasher.update(fp.architecture.as_bytes());
    if let Some(libc) = &fp.libc_version {
        hasher.update(libc.as_bytes());
    }
    hasher.update(fp.toolchain_hash.as_bytes());
    let mut compilers: Vec<(&String, &String)> = fp.compiler_versions.iter().collect();
    compilers.sort_by_key(|(name, _)| name.as_str());
    for (name, version) in compilers {
        hasher.update(name.as_bytes());
        hasher.update(version.as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::environment::EnvironmentFingerprint;
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn fingerprint(os: &str, compiler: &str) -> EnvironmentFingerprint {
        let mut compilers = HashMap::new();
        compilers.insert("rustc".to_string(), compiler.to_string());
        EnvironmentFingerprint {
            os: os.to_string(),
            os_version: "1.0".to_string(),
            architecture: "x86_64".to_string(),
            libc_version: None,
            compiler_versions: compilers,
            toolchain_hash: "abc".to_string(),
        }
    }

    #[test]
    fn test_first_run_then_stable() {
        let dir = tempdir().unwrap();
        let detector = DriftDetector::new(dir.path());
        let fp = fingerprint("linux", "1.88");

        assert!(matches!(
            detector.check_and_record(&fp).unwrap(),
            DriftVerdict::FirstRun
        ));
        assert!(matches!(
            detector.check_and_record(&fp).unwrap(),
            DriftVerdict::Stable
        ));
    }

    #[test]
    fn test_drift_detected_on_compiler_upgrade() {
        let dir = tempdir().unwrap();
        let detector = DriftDetector::new(dir.path());

        detector
            .check_and_record(&fingerprint("linux", "1.88"))
            .unwrap();
        let verdict = detector
            .check_and_record(&fingerprint("linux", "1.90"))
            .unwrap();

        match verdict {
            DriftVerdict::Drifted {
                previous_hash,
                current_hash,
            } => {
                assert_ne!(previous_hash, current_hash);
            }
            other => panic!("expected Drifted, got {other:?}"),
        }
    }

    #[test]
    fn test_os_change_also_triggers_drift() {
        let dir = tempdir().unwrap();
        let detector = DriftDetector::new(dir.path());
        detector
            .check_and_record(&fingerprint("linux", "1.88"))
            .unwrap();
        assert!(matches!(
            detector
                .check_and_record(&fingerprint("macos", "1.88"))
                .unwrap(),
            DriftVerdict::Drifted { .. }
        ));
    }
}
