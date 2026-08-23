#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildSnapshot {
    pub id: String,
    pub timestamp: u64,
    pub project_name: String,
    pub git_ref: Option<String>,
    pub artifacts: HashMap<String, String>,
    pub total_artifacts: usize,
}

/// Content-addressed build history.
///
/// Artifact bytes are stored verbatim under `.fish/history/blobs/<blake3-hex>`
/// when [`TimeMachine::store_artifact`] is called; snapshots reference those
/// hashes. Rewinding reads the stored bytes back and verifies their digest
/// before restoring, so a rewind never fabricates placeholder content.
pub struct TimeMachine {
    storage_dir: PathBuf,
}

impl TimeMachine {
    pub fn new(project_root: &Path) -> Self {
        let storage_dir = project_root.join(".fish").join("history");
        Self { storage_dir }
    }

    fn blob_dir(&self) -> PathBuf {
        self.storage_dir.join("blobs")
    }

    fn blob_path(&self, hash: &str) -> PathBuf {
        self.blob_dir().join(hash)
    }

    /// Persist artifact bytes under their BLAKE3 digest and return the hash.
    pub fn store_artifact(&self, bytes: &[u8]) -> io::Result<String> {
        let hash = blake3::hash(bytes).to_hex().to_string();
        fs::create_dir_all(self.blob_dir())?;
        let path = self.blob_path(&hash);
        if !path.exists() {
            fs::write(&path, bytes)?;
        }
        Ok(hash)
    }

    pub fn record_snapshot(
        &self,
        project_name: &str,
        artifacts: HashMap<String, String>,
    ) -> io::Result<BuildSnapshot> {
        fs::create_dir_all(&self.storage_dir)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let id = format!("snap_{:x}_{}", now, artifacts.len());
        let total_artifacts = artifacts.len();

        let snapshot = BuildSnapshot {
            id: id.clone(),
            timestamp: now,
            project_name: project_name.to_string(),
            git_ref: Self::detect_git_ref(),
            artifacts,
            total_artifacts,
        };

        let file_path = self.storage_dir.join(format!("{}.json", id));
        let content = serde_json::to_string_pretty(&snapshot)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        fs::write(file_path, content)?;

        Ok(snapshot)
    }

    pub fn list_snapshots(&self) -> io::Result<Vec<BuildSnapshot>> {
        if !self.storage_dir.exists() {
            return Ok(Vec::new());
        }

        let mut snapshots = Vec::new();
        for entry in fs::read_dir(&self.storage_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json")
                && let Ok(content) = fs::read_to_string(&path)
                && let Ok(snap) = serde_json::from_str::<BuildSnapshot>(&content)
            {
                snapshots.push(snap);
            }
        }

        snapshots.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
        Ok(snapshots)
    }

    /// Restore every artifact of `snapshot_id` into `destination_dir`.
    ///
    /// All referenced blobs are verified up front; if any blob is missing or
    /// fails its digest check the rewind aborts without touching the
    /// destination so callers never receive a half-restored tree.
    pub fn rewind_to_snapshot(
        &self,
        snapshot_id: &str,
        destination_dir: &Path,
    ) -> io::Result<usize> {
        let snapshots = self.list_snapshots()?;
        let target = snapshots
            .iter()
            .find(|s| s.id == snapshot_id || s.id.starts_with(snapshot_id))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Snapshot `{}` not found in history", snapshot_id),
                )
            })?;

        let mut pending: Vec<(PathBuf, String, Vec<u8>)> = Vec::new();
        for (rel_path, hash) in &target.artifacts {
            let bytes = fs::read(self.blob_path(hash)).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!(
                        "Blob {hash} for `{rel_path}` is not in history storage; \
                         the snapshot cannot be fully restored"
                    ),
                )
            })?;
            let computed = blake3::hash(&bytes).to_hex().to_string();
            if &computed != hash {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!(
                        "Blob {hash} failed its digest check (found {computed}); \
                         refusing to restore corrupted content"
                    ),
                ));
            }
            let dest_file = destination_dir.join(rel_path);
            pending.push((dest_file, hash.clone(), bytes));
        }

        fs::create_dir_all(destination_dir)?;
        let mut restored = 0;
        for (dest_file, _hash, bytes) in pending {
            if let Some(parent) = dest_file.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&dest_file, bytes)?;
            restored += 1;
        }

        Ok(restored)
    }

    fn detect_git_ref() -> Option<String> {
        std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_time_machine_record_list_and_rewind_restores_real_bytes() {
        let temp = tempdir().unwrap();
        let tm = TimeMachine::new(temp.path());

        let app_bytes = b"actual linked binary payload".to_vec();
        let lib_bytes = b"rlib archive bytes".to_vec();
        let app_hash = tm.store_artifact(&app_bytes).unwrap();
        let lib_hash = tm.store_artifact(&lib_bytes).unwrap();

        let mut artifacts = HashMap::new();
        artifacts.insert("bin/app.exe".to_string(), app_hash.clone());
        artifacts.insert("lib/core.rlib".to_string(), lib_hash);

        let snapshot = tm.record_snapshot("demo_project", artifacts).unwrap();
        assert_eq!(snapshot.total_artifacts, 2);

        let list = tm.list_snapshots().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, snapshot.id);

        let restore_dir = temp.path().join("restored_target");
        let count = tm.rewind_to_snapshot(&snapshot.id, &restore_dir).unwrap();
        assert_eq!(count, 2);
        assert_eq!(
            fs::read(restore_dir.join("bin/app.exe")).unwrap(),
            app_bytes
        );
        assert_eq!(
            fs::read(restore_dir.join("lib/core.rlib")).unwrap(),
            lib_bytes
        );
    }

    #[test]
    fn test_rewind_fails_loudly_when_blob_missing() {
        let temp = tempdir().unwrap();
        let tm = TimeMachine::new(temp.path());

        let mut artifacts = HashMap::new();
        artifacts.insert("bin/gone".to_string(), format!("{:064}", 'f'));
        let snapshot = tm.record_snapshot("demo", artifacts).unwrap();

        let result = tm.rewind_to_snapshot(&snapshot.id, &temp.path().join("out"));
        let err = result.expect_err("missing blob must abort the rewind");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(!temp.path().join("out").join("bin").exists());
    }

    #[test]
    fn test_rewind_rejects_corrupted_blob() {
        let temp = tempdir().unwrap();
        let tm = TimeMachine::new(temp.path());

        let good = b"payload".to_vec();
        let hash = tm.store_artifact(&good).unwrap();
        let mut artifacts = HashMap::new();
        artifacts.insert("artifact.bin".to_string(), hash.clone());
        let snapshot = tm.record_snapshot("demo", artifacts).unwrap();

        fs::write(tm.blob_path(&hash), b"tampered").unwrap();

        let result = tm.rewind_to_snapshot(&snapshot.id, &temp.path().join("out"));
        let err = result.expect_err("digest mismatch must abort the rewind");
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_store_artifact_is_deduplicated_by_content() {
        let temp = tempdir().unwrap();
        let tm = TimeMachine::new(temp.path());

        let h1 = tm.store_artifact(b"same").unwrap();
        let h2 = tm.store_artifact(b"same").unwrap();
        assert_eq!(h1, h2);

        let blobs = fs::read_dir(tm.blob_dir()).unwrap();
        assert_eq!(blobs.count(), 1);
    }
}
