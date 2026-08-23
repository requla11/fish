use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct CasGcConfig {
    pub max_storage_bytes: u64,
    pub prune_unreferenced: bool,
    pub run_interval: Duration,
}

impl Default for CasGcConfig {
    fn default() -> Self {
        Self {
            max_storage_bytes: 50 * 1024 * 1024 * 1024,
            prune_unreferenced: true,
            run_interval: Duration::from_secs(600),
        }
    }
}

/// Garbage collector for the sharded on-disk CAS layout
/// (`<root>/<2-hex shard>/<62-hex name>` plus `.meta` siblings).
///
/// Pruning is invoked explicitly by the embedder with the set of hashes that
/// are still referenced; `start`/`stop` only toggle the activity flag and do
/// not spawn a background thread.
pub struct CasGarbageCollector {
    storage_root: PathBuf,
    config: CasGcConfig,
    active: Arc<AtomicBool>,
}

fn is_shard_dir_name(name: &str) -> bool {
    name.len() == 2 && name.bytes().all(|b| b.is_ascii_hexdigit())
}

fn blob_file_name(file_name: &str) -> Option<&str> {
    if file_name.contains(".tmp.") || file_name.starts_with('.') {
        return None;
    }
    Some(file_name.strip_suffix(".meta").unwrap_or(file_name))
}

impl CasGarbageCollector {
    pub fn new(storage_root: PathBuf, config: CasGcConfig) -> Self {
        Self {
            storage_root,
            config,
            active: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn config(&self) -> &CasGcConfig {
        &self.config
    }

    pub fn start(&self) {
        self.active.store(true, Ordering::SeqCst);
    }

    pub fn stop(&self) {
        self.active.store(false, Ordering::SeqCst);
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    /// Remove every data/metadata pair whose reconstructed hash is absent from
    /// `referenced_hashes`. Returns `(files removed, bytes freed)` including
    /// metadata files in the count.
    pub fn prune_unreferenced_blobs(&self, referenced_hashes: &HashSet<String>) -> (usize, u64) {
        let mut pruned = 0;
        let mut freed = 0;
        let root = self.storage_root.clone();
        visit_shards(&root, referenced_hashes, &mut pruned, &mut freed);
        (pruned, freed)
    }
}

fn visit_shards(root: &Path, referenced: &HashSet<String>, pruned: &mut usize, freed: &mut u64) {
    let Ok(shards) = std::fs::read_dir(root) else {
        return;
    };
    for shard in shards.flatten() {
        let shard_path = shard.path();
        if !shard_path.is_dir() {
            continue;
        }
        let Some(shard_name) = shard_path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !is_shard_dir_name(shard_name) {
            continue;
        }
        prune_shard(&shard_path, shard_name, referenced, pruned, freed);
    }
}

fn prune_shard(
    shard_path: &Path,
    shard_name: &str,
    referenced: &HashSet<String>,
    pruned: &mut usize,
    freed: &mut u64,
) {
    let Ok(entries) = std::fs::read_dir(shard_path) else {
        return;
    };
    let to_remove: Vec<(PathBuf, u64)> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_file() {
                return None;
            }
            let file_name = path.file_name()?.to_str()?;
            let stem = blob_file_name(file_name)?;
            let full_hash = format!("{shard_name}{stem}");
            if referenced.contains(&full_hash) {
                return None;
            }
            let size = std::fs::metadata(&path).ok()?.len();
            Some((path, size))
        })
        .collect();
    for (path, size) in to_remove {
        if std::fs::remove_file(&path).is_ok() {
            *pruned += 1;
            *freed += size;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cas_gc_pruning_matches_sharded_layout() {
        let temp = tempdir().unwrap();
        let shard_ab = temp.path().join("ab");
        let shard_cd = temp.path().join("cd");
        std::fs::create_dir_all(&shard_ab).unwrap();
        std::fs::create_dir_all(&shard_cd).unwrap();

        let keep_data = shard_ab.join("c".repeat(62));
        let keep_meta = shard_ab.join(format!("{}.meta", "c".repeat(62)));
        let drop_data = shard_cd.join("d".repeat(62));
        let drop_meta = shard_cd.join(format!("{}.meta", "d".repeat(62)));
        std::fs::write(&keep_data, b"content_keep").unwrap();
        std::fs::write(&keep_meta, "{}").unwrap();
        std::fs::write(&drop_data, b"content_drop").unwrap();
        std::fs::write(&drop_meta, "{}").unwrap();

        let tmp_junk = shard_ab.join(format!("{}.tmp.123", "e".repeat(62)));
        std::fs::write(&tmp_junk, b"partial write").unwrap();

        let mut referenced = HashSet::new();
        referenced.insert(format!("ab{}", "c".repeat(62)));

        let gc = CasGarbageCollector::new(temp.path().to_path_buf(), CasGcConfig::default());
        let (pruned, freed) = gc.prune_unreferenced_blobs(&referenced);

        assert_eq!(pruned, 2);
        assert_eq!(freed, "content_drop".len() as u64 + "{}".len() as u64);
        assert!(keep_data.exists());
        assert!(keep_meta.exists());
        assert!(!drop_data.exists());
        assert!(!drop_meta.exists());
        assert!(tmp_junk.exists(), "in-progress temp writes must survive GC");
    }

    #[test]
    fn test_cas_gc_ignores_flat_layout_and_non_shard_dirs() {
        let temp = tempdir().unwrap();
        let legacy_objects = temp.path().join("objects");
        std::fs::create_dir_all(&legacy_objects).unwrap();
        let stray = temp.path().join("zz");
        std::fs::create_dir_all(&stray).unwrap();
        std::fs::write(legacy_objects.join("hash_a"), b"data").unwrap();
        std::fs::write(stray.join("hash_b"), b"data").unwrap();

        let gc = CasGarbageCollector::new(temp.path().to_path_buf(), CasGcConfig::default());
        let (pruned, _) = gc.prune_unreferenced_blobs(&HashSet::new());

        assert_eq!(pruned, 0);
        assert!(legacy_objects.join("hash_a").exists());
        assert!(stray.join("hash_b").exists());
    }

    #[test]
    fn test_blob_file_name_rejects_tmp_and_hidden() {
        assert_eq!(
            blob_file_name(&format!("{}.meta", "a".repeat(62))),
            Some("a".repeat(62).as_str())
        );
        assert_eq!(
            blob_file_name(&"a".repeat(62)),
            Some("a".repeat(62).as_str())
        );
        assert_eq!(blob_file_name("abc.tmp.999"), None);
        assert_eq!(blob_file_name(".hidden"), None);
    }
}
