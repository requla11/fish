#![allow(unsafe_code)]

use crate::artifact::{ArtifactHash, ArtifactMetadata};
use crate::compression::CompressionAlgorithm;
use crate::error::{CasError, Result};
use memmap2::{Mmap, MmapOptions};
use std::fs::File;
use std::ops::Deref;
use std::path::Path;
use std::str::FromStr;

pub struct MmapArtifact {
    metadata: ArtifactMetadata,
    mmap: Option<Mmap>,
    decompressed: Option<Vec<u8>>,
}

impl MmapArtifact {
    pub fn open(data_path: &Path, metadata: ArtifactMetadata) -> Result<Self> {
        let file = File::open(data_path).map_err(CasError::Io)?;
        let file_len = file.metadata().map_err(CasError::Io)?.len();

        if file_len == 0 {
            let computed = ArtifactHash::from_bytes(&[])?;
            if &computed != &metadata.hash {
                return Err(CasError::Hash(format!(
                    "mmap empty artifact hash mismatch: declared {}, got {}",
                    metadata.hash, computed
                )));
            }
            return Ok(Self {
                metadata,
                mmap: None,
                decompressed: None,
            });
        }

        let mmap = unsafe { MmapOptions::new().map(&file).map_err(CasError::Io)? };

        if let Some(ref algo_str) = metadata.compression {
            if let Ok(algo) = CompressionAlgorithm::from_str(algo_str) {
                if algo != CompressionAlgorithm::None {
                    let decompressed = crate::compression::decompress(&mmap, algo)?;
                    let computed = ArtifactHash::from_bytes(&decompressed)?;
                    if &computed != &metadata.hash {
                        return Err(CasError::Hash(format!(
                            "mmap decompressed artifact hash mismatch: declared {}, got {}",
                            metadata.hash, computed
                        )));
                    }
                    return Ok(Self {
                        metadata,
                        mmap: Some(mmap),
                        decompressed: Some(decompressed),
                    });
                }
            }
        }

        let computed = ArtifactHash::from_bytes(&mmap)?;
        if &computed != &metadata.hash {
            return Err(CasError::Hash(format!(
                "mmap artifact hash mismatch: declared {}, got {}",
                metadata.hash, computed
            )));
        }

        Ok(Self {
            metadata,
            mmap: Some(mmap),
            decompressed: None,
        })
    }

    pub fn metadata(&self) -> &ArtifactMetadata {
        &self.metadata
    }

    pub fn hash(&self) -> &ArtifactHash {
        &self.metadata.hash
    }

    pub fn as_slice(&self) -> &[u8] {
        if let Some(ref decompressed) = self.decompressed {
            decompressed.as_slice()
        } else if let Some(ref mmap) = self.mmap {
            &mmap[..]
        } else {
            &[]
        }
    }

    pub fn is_zero_copy(&self) -> bool {
        self.decompressed.is_none()
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }
}

impl Deref for MmapArtifact {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl AsRef<[u8]> for MmapArtifact {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_mmap_uncompressed_artifact() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.bin");
        let data = b"zero copy memory mapped payload content";
        std::fs::write(&file_path, data).unwrap();

        let hash = ArtifactHash::from_bytes(data).unwrap();
        let metadata = ArtifactMetadata::new(
            hash,
            data.len() as u64,
            "bin".to_string(),
            "test".to_string(),
        );

        let artifact = MmapArtifact::open(&file_path, metadata).unwrap();
        assert!(artifact.is_zero_copy());
        assert_eq!(artifact.len(), data.len());
        assert_eq!(artifact.as_slice(), data);
        assert_eq!(&artifact[..], data);
    }

    #[test]
    fn test_mmap_empty_artifact() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("empty.bin");
        std::fs::write(&file_path, b"").unwrap();

        let hash = ArtifactHash::from_bytes(b"").unwrap();
        let metadata = ArtifactMetadata::new(hash, 0, "bin".to_string(), "test".to_string());

        let artifact = MmapArtifact::open(&file_path, metadata).unwrap();
        assert!(artifact.is_zero_copy());
        assert_eq!(artifact.len(), 0);
        assert!(artifact.is_empty());
        assert_eq!(artifact.as_slice(), b"");
    }

    #[test]
    fn test_mmap_compressed_artifact() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("compressed.bin");
        let original_data = b"compressed payload with high repetition aaaaaaaaaaaaaaaaaaaaaaaa";
        let compressed_data =
            crate::compression::compress(original_data, CompressionAlgorithm::Zstd).unwrap();
        std::fs::write(&file_path, &compressed_data).unwrap();

        let hash = ArtifactHash::from_bytes(original_data).unwrap();
        let mut metadata = ArtifactMetadata::new(
            hash,
            original_data.len() as u64,
            "bin".to_string(),
            "test".to_string(),
        );
        metadata = metadata.with_compression(compressed_data.len() as u64, "zstd".to_string());

        let artifact = MmapArtifact::open(&file_path, metadata).unwrap();
        assert!(!artifact.is_zero_copy());
        assert_eq!(artifact.as_slice(), original_data);
    }

    #[test]
    fn test_mmap_hash_mismatch() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("corrupted.bin");
        std::fs::write(&file_path, b"actual content").unwrap();

        let fake_hash = ArtifactHash::from_bytes(b"expected different content").unwrap();
        let metadata = ArtifactMetadata::new(fake_hash, 14, "bin".to_string(), "test".to_string());

        let err = MmapArtifact::open(&file_path, metadata);
        assert!(err.is_err());
    }
}
