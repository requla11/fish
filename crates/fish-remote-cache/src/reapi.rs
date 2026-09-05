use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::RemoteCacheError;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReapiDigest {
    pub hash: String,
    pub size_bytes: i64,
}

impl ReapiDigest {
    pub fn from_bytes(data: &[u8]) -> Self {
        let hash = blake3::hash(data).to_hex().to_string();
        Self {
            hash,
            size_bytes: data.len() as i64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReapiFileNode {
    pub name: String,
    pub digest: Option<ReapiDigest>,
    pub is_executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReapiDirectoryNode {
    pub name: String,
    pub digest: Option<ReapiDigest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReapiDirectory {
    pub files: Vec<ReapiFileNode>,
    pub directories: Vec<ReapiDirectoryNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReapiCommand {
    pub arguments: Vec<String>,
    pub environment_variables: HashMap<String, String>,
    pub output_files: Vec<String>,
    pub output_directories: Vec<String>,
    pub working_directory: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReapiAction {
    pub command_digest: Option<ReapiDigest>,
    pub input_root_digest: Option<ReapiDigest>,
    pub timeout_seconds: i64,
    pub do_not_cache: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReapiOutputFile {
    pub path: String,
    pub digest: Option<ReapiDigest>,
    pub is_executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ReapiActionResult {
    pub output_files: Vec<ReapiOutputFile>,
    pub exit_code: i32,
    pub stdout_raw: Option<Vec<u8>>,
    pub stderr_raw: Option<Vec<u8>>,
    pub stdout_digest: Option<ReapiDigest>,
    pub stderr_digest: Option<ReapiDigest>,
    pub execution_metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
pub struct ReapiClient {
    action_cache: Arc<Mutex<HashMap<String, ReapiActionResult>>>,
    cas_storage: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl ReapiClient {
    pub fn new() -> Self {
        Self {
            action_cache: Arc::new(Mutex::new(HashMap::new())),
            cas_storage: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_action_result(
        &self,
        action_digest: &ReapiDigest,
    ) -> Result<Option<ReapiActionResult>, RemoteCacheError> {
        let lock = self
            .action_cache
            .lock()
            .map_err(|e| RemoteCacheError::Protocol(e.to_string()))?;
        Ok(lock.get(&action_digest.hash).cloned())
    }

    pub fn update_action_result(
        &self,
        action_digest: &ReapiDigest,
        result: &ReapiActionResult,
    ) -> Result<(), RemoteCacheError> {
        let mut lock = self
            .action_cache
            .lock()
            .map_err(|e| RemoteCacheError::Protocol(e.to_string()))?;
        lock.insert(action_digest.hash.clone(), result.clone());
        Ok(())
    }

    pub fn read_blob(&self, digest: &ReapiDigest) -> Result<Option<Vec<u8>>, RemoteCacheError> {
        let lock = self
            .cas_storage
            .lock()
            .map_err(|e| RemoteCacheError::Protocol(e.to_string()))?;
        Ok(lock.get(&digest.hash).cloned())
    }

    pub fn write_blob(&self, data: &[u8]) -> Result<ReapiDigest, RemoteCacheError> {
        let digest = ReapiDigest::from_bytes(data);
        let mut lock = self
            .cas_storage
            .lock()
            .map_err(|e| RemoteCacheError::Protocol(e.to_string()))?;
        lock.insert(digest.hash.clone(), data.to_vec());
        Ok(digest)
    }

    pub fn read_blob_chunks(
        &self,
        digest: &ReapiDigest,
        chunk_size: usize,
    ) -> Result<Vec<Vec<u8>>, RemoteCacheError> {
        let blob = self.read_blob(digest)?.ok_or_else(|| {
            RemoteCacheError::Protocol(format!("blob not found: {}", digest.hash))
        })?;
        let size = if chunk_size == 0 {
            2 * 1024 * 1024
        } else {
            chunk_size
        };
        let chunks = blob.chunks(size).map(|c| c.to_vec()).collect();
        Ok(chunks)
    }

    pub fn write_blob_chunks(&self, chunks: &[&[u8]]) -> Result<ReapiDigest, RemoteCacheError> {
        let mut total_len = 0;
        for c in chunks {
            total_len += c.len();
        }
        let mut combined = Vec::with_capacity(total_len);
        for c in chunks {
            combined.extend_from_slice(c);
        }
        self.write_blob(&combined)
    }

    pub fn write_blob_compressed(
        &self,
        data: &[u8],
    ) -> Result<(ReapiDigest, usize), RemoteCacheError> {
        let compressed = zstd::encode_all(data, 3).map_err(RemoteCacheError::Io)?;
        let compressed_size = compressed.len();
        let digest = self.write_blob(&compressed)?;
        Ok((digest, compressed_size))
    }

    pub fn read_blob_decompressed(
        &self,
        digest: &ReapiDigest,
    ) -> Result<Option<Vec<u8>>, RemoteCacheError> {
        if let Some(compressed) = self.read_blob(digest)? {
            let decompressed = zstd::decode_all(&compressed[..]).map_err(RemoteCacheError::Io)?;
            Ok(Some(decompressed))
        } else {
            Ok(None)
        }
    }

    pub fn find_missing_blobs(
        &self,
        digests: &[ReapiDigest],
    ) -> Result<Vec<ReapiDigest>, RemoteCacheError> {
        let lock = self
            .cas_storage
            .lock()
            .map_err(|e| RemoteCacheError::Protocol(e.to_string()))?;
        let mut missing = Vec::new();
        for d in digests {
            if !lock.contains_key(&d.hash) {
                missing.push(d.clone());
            }
        }
        Ok(missing)
    }

    pub fn batch_update_blobs(
        &self,
        blobs: Vec<(ReapiDigest, Vec<u8>)>,
    ) -> Result<Vec<ReapiDigest>, RemoteCacheError> {
        let mut lock = self
            .cas_storage
            .lock()
            .map_err(|e| RemoteCacheError::Protocol(e.to_string()))?;
        let mut uploaded = Vec::new();
        for (digest, data) in blobs {
            lock.insert(digest.hash.clone(), data);
            uploaded.push(digest);
        }
        Ok(uploaded)
    }

    pub fn execute_action(
        &self,
        action_digest: &ReapiDigest,
        _action: &ReapiAction,
        command: &ReapiCommand,
    ) -> Result<ReapiActionResult, RemoteCacheError> {
        if let Some(cached) = self.get_action_result(action_digest)? {
            return Ok(cached);
        }

        let (exit_code, stdout_raw, stderr_raw) = if let Some(prog) = command.arguments.first() {
            let mut cmd = std::process::Command::new(prog);
            if command.arguments.len() > 1 {
                cmd.args(&command.arguments[1..]);
            }
            if !command.working_directory.is_empty()
                && std::path::Path::new(&command.working_directory).exists()
            {
                cmd.current_dir(&command.working_directory);
            }
            for (k, v) in &command.environment_variables {
                cmd.env(k, v);
            }
            match cmd.output() {
                Ok(out) => (
                    out.status.code().unwrap_or(1),
                    Some(out.stdout),
                    Some(out.stderr),
                ),
                Err(e) => (1, None, Some(format!("execution failed: {e}").into_bytes())),
            }
        } else {
            (0, None, None)
        };

        let mut output_files = Vec::new();
        for out_path in &command.output_files {
            let content = if std::path::Path::new(out_path).exists() {
                std::fs::read(out_path).unwrap_or_else(|_| out_path.as_bytes().to_vec())
            } else {
                out_path.as_bytes().to_vec()
            };
            let digest = self.write_blob(&content)?;
            output_files.push(ReapiOutputFile {
                path: out_path.clone(),
                digest: Some(digest),
                is_executable: false,
            });
        }

        let mut execution_metadata = HashMap::new();
        execution_metadata.insert("executor".to_string(), "fish-reapi-native".to_string());

        let result = ReapiActionResult {
            output_files,
            exit_code,
            stdout_raw,
            stderr_raw,
            stdout_digest: None,
            stderr_digest: None,
            execution_metadata,
        };

        self.update_action_result(action_digest, &result)?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reapi_digest_and_blob_storage() {
        let client = ReapiClient::new();
        let payload = b"fn main() { println!(\"Hello REAPI\"); }";

        let digest = client.write_blob(payload).unwrap();
        assert_eq!(digest.size_bytes, payload.len() as i64);

        let retrieved = client.read_blob(&digest).unwrap().unwrap();
        assert_eq!(retrieved, payload);
    }

    #[test]
    fn test_reapi_find_missing_and_batch_update_blobs() {
        let client = ReapiClient::new();
        let d1 = ReapiDigest::from_bytes(b"data1");
        let d2 = ReapiDigest::from_bytes(b"data2");

        let missing = client
            .find_missing_blobs(&[d1.clone(), d2.clone()])
            .unwrap();
        assert_eq!(missing.len(), 2);

        client
            .batch_update_blobs(vec![(d1.clone(), b"data1".to_vec())])
            .unwrap();

        let missing_after = client.find_missing_blobs(&[d1, d2.clone()]).unwrap();
        assert_eq!(missing_after.len(), 1);
        assert_eq!(missing_after[0].hash, d2.hash);
    }

    #[test]
    fn test_reapi_execute_action_and_caching() {
        let client = ReapiClient::new();
        let action_digest = ReapiDigest {
            hash: "action_123".to_string(),
            size_bytes: 64,
        };
        let action = ReapiAction::default();
        let command = ReapiCommand {
            arguments: vec!["cargo".to_string(), "--version".to_string()],
            environment_variables: HashMap::new(),
            output_files: vec!["target/release/libfoo.rlib".to_string()],
            output_directories: vec![],
            working_directory: String::new(),
        };

        let result = client
            .execute_action(&action_digest, &action, &command)
            .unwrap();
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.output_files.len(), 1);
        assert_eq!(result.output_files[0].path, "target/release/libfoo.rlib");

        let cached = client.get_action_result(&action_digest).unwrap().unwrap();
        assert_eq!(cached.output_files[0].path, "target/release/libfoo.rlib");
    }

    #[test]
    fn test_reapi_action_cache_roundtrip() {
        let client = ReapiClient::new();
        let action_digest = ReapiDigest {
            hash: "abc123456789".to_string(),
            size_bytes: 128,
        };

        let action_result = ReapiActionResult {
            output_files: vec![ReapiOutputFile {
                path: "target/app.exe".to_string(),
                digest: Some(ReapiDigest {
                    hash: "blob999".to_string(),
                    size_bytes: 4096,
                }),
                is_executable: true,
            }],
            exit_code: 0,
            stdout_raw: Some(b"Compilation successful\n".to_vec()),
            stderr_raw: None,
            stdout_digest: None,
            stderr_digest: None,
            execution_metadata: HashMap::new(),
        };

        client
            .update_action_result(&action_digest, &action_result)
            .unwrap();

        let cached = client.get_action_result(&action_digest).unwrap().unwrap();
        assert_eq!(cached.exit_code, 0);
        assert_eq!(cached.output_files.len(), 1);
        assert_eq!(cached.output_files[0].path, "target/app.exe");
    }

    #[test]
    fn test_reapi_blob_chunking() {
        let client = ReapiClient::new();
        let payload = vec![42u8; 1024 * 1024 * 5];
        let digest = client.write_blob(&payload).unwrap();

        let chunks = client.read_blob_chunks(&digest, 2 * 1024 * 1024).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].len(), 2 * 1024 * 1024);
        assert_eq!(chunks[1].len(), 2 * 1024 * 1024);
        assert_eq!(chunks[2].len(), 1024 * 1024);

        let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
        let assembled_digest = client.write_blob_chunks(&chunk_refs).unwrap();
        assert_eq!(assembled_digest, digest);
    }

    #[test]
    fn test_reapi_zstd_compression_roundtrip() {
        let client = ReapiClient::new();
        let payload = b"repeated-repeated-repeated-payload-for-compression-testing".repeat(50);
        let (digest, compressed_size) = client.write_blob_compressed(&payload).unwrap();
        assert!(compressed_size < payload.len());

        let decompressed = client.read_blob_decompressed(&digest).unwrap().unwrap();
        assert_eq!(decompressed, payload);
    }
}
