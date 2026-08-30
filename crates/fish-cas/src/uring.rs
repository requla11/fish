#![deny(unsafe_code)]

//! io_uring accelerated file I/O for high-fanout cache fetch storms
//!
//! ROADMAP.md:84 - io_uring Async Executor Backend
//! When the `io-uring` feature is enabled on Linux, this module routes
//! hot-path CAS reads/writes through `tokio-uring` submission queues for
//! zero-copy, batched completions. On other platforms or without the feature
//! it transparently falls back to `tokio::fs`.

use crate::error::{CasError, Result};
use std::path::Path;

/// Read a file via io_uring when available, fallback to tokio::fs otherwise.
#[cfg(all(target_os = "linux", feature = "io-uring"))]
pub async fn read_file_uring(path: &Path) -> Result<Vec<u8>> {
    let path_buf = path.to_path_buf();
    // tokio-uring requires its own runtime; spawn_blocking avoids nesting inside a `tokio` runtime.
    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move {
            let file = tokio_uring::fs::File::open(&path_buf)
                .await
                .map_err(CasError::Io)?;
            let mut buf = Vec::new();
            let mut offset: u64 = 0;
            loop {
                let chunk = vec![0u8; 64 * 1024];
                let (res, chunk) = file.read_at(chunk, offset).await;
                let n = res.map_err(CasError::Io)?;
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&chunk[..n]);
                offset += n as u64;
                if n < chunk.len() {
                    break;
                }
            }
            Ok::<Vec<u8>, CasError>(buf)
        })
    })
    .await
    .map_err(|e| CasError::Io(std::io::Error::other(e.to_string())))?
}

#[cfg(not(all(target_os = "linux", feature = "io-uring")))]
pub async fn read_file_uring(path: &Path) -> Result<Vec<u8>> {
    tokio::fs::read(path).await.map_err(CasError::Io)
}

/// Write a file via io_uring when available, fallback to tokio::fs otherwise.
#[cfg(all(target_os = "linux", feature = "io-uring"))]
pub async fn write_file_uring(path: &Path, data: &[u8]) -> Result<()> {
    let path_buf = path.to_path_buf();
    let data_vec = data.to_vec();
    tokio::task::spawn_blocking(move || {
        tokio_uring::start(async move {
            let file = tokio_uring::fs::File::create(&path_buf)
                .await
                .map_err(CasError::Io)?;
            let (res, _) = file.write_at(data_vec, 0).await;
            res.map_err(CasError::Io)?;
            file.close().await.map_err(CasError::Io)?;
            Ok::<(), CasError>(())
        })
    })
    .await
    .map_err(|e| CasError::Io(std::io::Error::other(e.to_string())))?
}

#[cfg(not(all(target_os = "linux", feature = "io-uring")))]
pub async fn write_file_uring(path: &Path, data: &[u8]) -> Result<()> {
    tokio::fs::write(path, data).await.map_err(CasError::Io)
}

/// Check if io_uring backend is active at compile time.
pub const fn is_uring_enabled() -> bool {
    cfg!(all(target_os = "linux", feature = "io-uring"))
}
