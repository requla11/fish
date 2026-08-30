#![deny(unsafe_code)]

//! io_uring accelerated file I/O for cache fetch storms
//!
//! Mirrors `fish-cas/src/uring.rs` but for `LocalCache` file operations.
//! On Linux with `--features io-uring` this uses `tokio-uring` submission
//! queues; elsewhere it falls back to `tokio::fs` (cache ops are currently sync
//! so this module also exposes a blocking fallback).

use crate::CacheError;
use std::path::Path;

/// Async read via io_uring when available.
#[cfg(all(target_os = "linux", feature = "io-uring"))]
pub async fn read_file_uring(path: &Path) -> Result<Vec<u8>, CacheError> {
    let path_buf = path.to_path_buf();
    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move {
            let file =
                tokio_uring::fs::File::open(&path_buf)
                    .await
                    .map_err(|e| CacheError::Read {
                        key: path_buf.display().to_string(),
                        source: e,
                    })?;
            let mut buf = Vec::new();
            let mut offset: u64 = 0;
            loop {
                let chunk = vec![0u8; 64 * 1024];
                let (res, chunk) = file.read_at(chunk, offset).await;
                let n = res.map_err(|e| CacheError::Read {
                    key: path_buf.display().to_string(),
                    source: e,
                })?;
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
                offset += n as u64;
                if n < chunk.len() {
                    break;
                }
            }
            Ok::<Vec<u8>, CacheError>(buf)
        })
    })
    .await
    .map_err(|e| CacheError::Read {
        key: path.display().to_string(),
        source: std::io::Error::other(e.to_string()),
    })?
}

#[cfg(not(all(target_os = "linux", feature = "io-uring")))]
pub async fn read_file_uring(path: &Path) -> Result<Vec<u8>, CacheError> {
    tokio::fs::read(path)
        .await
        .map_err(|source| CacheError::Read {
            key: path.display().to_string(),
            source,
        })
}

/// Async write via io_uring when available.
#[cfg(all(target_os = "linux", feature = "io-uring"))]
pub async fn write_file_uring(path: &Path, data: &[u8]) -> Result<(), CacheError> {
    let path_buf = path.to_path_buf();
    let data_vec = data.to_vec();
    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move {
            let file = tokio_uring::fs::File::create(&path_buf)
                .await
                .map_err(|e| CacheError::Write {
                    key: path_buf.display().to_string(),
                    source: e,
                })?;
            let (res, _) = file.write_at(data_vec, 0).await;
            res.map_err(|e| CacheError::Write {
                key: path_buf.display().to_string(),
                source: e,
            })?;
            file.close().await.map_err(|e| CacheError::Write {
                key: path_buf.display().to_string(),
                source: e,
            })?;
            Ok::<(), CacheError>(())
        })
    })
    .await
    .map_err(|e| CacheError::Write {
        key: path.display().to_string(),
        source: std::io::Error::other(e.to_string()),
    })?
}

#[cfg(not(all(target_os = "linux", feature = "io-uring")))]
pub async fn write_file_uring(path: &Path, data: &[u8]) -> Result<(), CacheError> {
    tokio::fs::write(path, data)
        .await
        .map_err(|source| CacheError::Write {
            key: path.display().to_string(),
            source,
        })
}

pub const fn is_uring_enabled() -> bool {
    cfg!(all(target_os = "linux", feature = "io-uring"))
}
