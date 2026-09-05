#![forbid(unsafe_code)]

use std::fmt::Debug;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::time::Duration;

use base64::Engine;

use crate::protocol::{CacheRequest, CacheResponse};
use crate::{RemoteCacheClient, RemoteCacheError};

#[derive(Debug, Clone)]
pub struct TcpRemoteCacheClient {
    pub server_addr: String,
    pub auth_token: Option<String>,
    pub timeout: Duration,
    pub offline: bool,
}

impl TcpRemoteCacheClient {
    pub fn new(server_addr: impl Into<String>, auth_token: Option<String>) -> Self {
        let offline = std::env::var("FISH_OFFLINE")
            .map(|v| {
                v == "1"
                    || v.eq_ignore_ascii_case("true")
                    || v.eq_ignore_ascii_case("yes")
                    || v.eq_ignore_ascii_case("on")
            })
            .unwrap_or(false);
        Self {
            server_addr: server_addr.into(),
            auth_token,
            timeout: Duration::from_secs(5),
            offline,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    fn send_request(&self, req: CacheRequest) -> Result<CacheResponse, RemoteCacheError> {
        let env_offline = std::env::var("FISH_OFFLINE")
            .map(|v| {
                v == "1"
                    || v.eq_ignore_ascii_case("true")
                    || v.eq_ignore_ascii_case("yes")
                    || v.eq_ignore_ascii_case("on")
            })
            .unwrap_or(false);
        if self.offline || env_offline {
            return Err(RemoteCacheError::Offline(
                "offline mode enabled (FISH_OFFLINE); remote cache access rejected".to_string(),
            ));
        }

        let mut last_err = RemoteCacheError::Network("failed to connect".to_string());
        for attempt in 0..3 {
            if attempt > 0 {
                std::thread::sleep(Duration::from_millis(50));
            }
            let mut stream = match TcpStream::connect(&self.server_addr) {
                Ok(s) => s,
                Err(e) => {
                    last_err = RemoteCacheError::Network(format!(
                        "cannot connect to {}: {e}",
                        self.server_addr
                    ));
                    continue;
                }
            };

            let _ = stream.set_read_timeout(Some(self.timeout));
            let _ = stream.set_write_timeout(Some(self.timeout));

            let req_json = serde_json::to_vec(&req).map_err(|e| {
                RemoteCacheError::Protocol(format!("cannot serialize request: {e}"))
            })?;

            if let Err(e) = stream.write_all(&req_json) {
                last_err = RemoteCacheError::Network(format!("failed to send request: {e}"));
                continue;
            }
            if let Err(e) = stream.write_all(b"\n") {
                last_err = RemoteCacheError::Network(format!("failed to send newline: {e}"));
                continue;
            }
            if let Err(e) = stream.flush() {
                last_err = RemoteCacheError::Network(format!("failed to flush stream: {e}"));
                continue;
            }

            let mut reader = BufReader::new(stream);
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    last_err = RemoteCacheError::Protocol("empty response from server".to_string());
                    continue;
                }
                Ok(_) => {
                    let resp: CacheResponse = serde_json::from_str(line.trim()).map_err(|e| {
                        RemoteCacheError::Protocol(format!("cannot parse response: {e}"))
                    })?;
                    return Ok(resp);
                }
                Err(e) => {
                    last_err = RemoteCacheError::Network(format!("failed to read response: {e}"));
                    continue;
                }
            }
        }
        Err(last_err)
    }

    pub fn ping(&self) -> Result<bool, RemoteCacheError> {
        let req = CacheRequest::Ping {
            auth_token: self.auth_token.clone(),
        };
        match self.send_request(req)? {
            CacheResponse::Pong { status, error, .. } => {
                if let Some(err) = error {
                    Err(RemoteCacheError::Protocol(err))
                } else {
                    Ok(status == "ok")
                }
            }
            CacheResponse::Error { message } => Err(RemoteCacheError::Protocol(message)),
            _ => Err(RemoteCacheError::Protocol(
                "unexpected response to ping".to_string(),
            )),
        }
    }

    pub fn get_artifact(&self, hash: &str) -> Result<Option<Vec<u8>>, RemoteCacheError> {
        let req = CacheRequest::GetArtifact {
            hash: hash.to_string(),
            auth_token: self.auth_token.clone(),
        };
        match self.send_request(req)? {
            CacheResponse::Artifact {
                found,
                data_base64,
                error,
                ..
            } => {
                if let Some(err) = error {
                    return Err(RemoteCacheError::Protocol(err));
                }
                if !found {
                    return Ok(None);
                }
                if let Some(encoded) = data_base64 {
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(&encoded)
                        .map_err(|e| {
                            RemoteCacheError::Protocol(format!("invalid base64 artifact: {e}"))
                        })?;
                    Ok(Some(bytes))
                } else {
                    Ok(None)
                }
            }
            CacheResponse::Error { message } => Err(RemoteCacheError::Protocol(message)),
            _ => Err(RemoteCacheError::Protocol(
                "unexpected response to get_artifact".to_string(),
            )),
        }
    }

    pub fn put_artifact(&self, hash: &str, data: &[u8]) -> Result<(), RemoteCacheError> {
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        let req = CacheRequest::PutArtifact {
            hash: hash.to_string(),
            data_base64: encoded,
            auth_token: self.auth_token.clone(),
        };

        match self.send_request(req)? {
            CacheResponse::PutArtifactResult { success, error, .. } => {
                if let Some(err) = error {
                    Err(RemoteCacheError::Protocol(err))
                } else if success {
                    Ok(())
                } else {
                    Err(RemoteCacheError::Protocol(
                        "failed to put artifact".to_string(),
                    ))
                }
            }
            CacheResponse::Error { message } => Err(RemoteCacheError::Protocol(message)),
            _ => Err(RemoteCacheError::Protocol(
                "unexpected response to put_artifact".to_string(),
            )),
        }
    }
}

impl RemoteCacheClient for TcpRemoteCacheClient {
    fn get_fingerprint(&self, key: &str) -> Result<Option<String>, RemoteCacheError> {
        let req = CacheRequest::GetFingerprint {
            key: key.to_string(),
            auth_token: self.auth_token.clone(),
        };

        match self.send_request(req)? {
            CacheResponse::Fingerprint {
                found,
                fingerprint,
                error,
                ..
            } => {
                if let Some(err) = error {
                    Err(RemoteCacheError::Protocol(err))
                } else if found {
                    Ok(fingerprint)
                } else {
                    Ok(None)
                }
            }
            CacheResponse::Error { message } => Err(RemoteCacheError::Protocol(message)),
            _ => Err(RemoteCacheError::Protocol(
                "unexpected response to get_fingerprint".to_string(),
            )),
        }
    }

    fn put_fingerprint(&self, key: &str, fingerprint: &str) -> Result<(), RemoteCacheError> {
        let req = CacheRequest::PutFingerprint {
            key: key.to_string(),
            fingerprint: fingerprint.to_string(),
            auth_token: self.auth_token.clone(),
        };

        match self.send_request(req)? {
            CacheResponse::PutFingerprintResult { success, error, .. } => {
                if let Some(err) = error {
                    Err(RemoteCacheError::Protocol(err))
                } else if success {
                    Ok(())
                } else {
                    Err(RemoteCacheError::Protocol(
                        "failed to put fingerprint".to_string(),
                    ))
                }
            }
            CacheResponse::Error { message } => Err(RemoteCacheError::Protocol(message)),
            _ => Err(RemoteCacheError::Protocol(
                "unexpected response to put_fingerprint".to_string(),
            )),
        }
    }

    fn get_artifact(&self, key: &str) -> Result<Option<Vec<u8>>, RemoteCacheError> {
        self.get_artifact(key)
    }

    fn put_artifact(&self, key: &str, data: &[u8]) -> Result<(), RemoteCacheError> {
        self.put_artifact(key, data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RemoteCacheServer;
    use tempfile::tempdir;

    #[test]
    fn test_offline_client_fail_fast() {
        let client = TcpRemoteCacheClient::new("127.0.0.1:9999", None).with_offline(true);
        let err = client.get_fingerprint("test_key").unwrap_err();
        match err {
            RemoteCacheError::Offline(msg) => {
                assert!(msg.contains("offline mode"));
            }
            other => panic!("expected RemoteCacheError::Offline, got {:?}", other),
        }
    }

    #[test]
    fn test_tcp_client_trait_artifact_roundtrip() {
        let temp = tempdir().unwrap();
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        drop(listener);

        let server = RemoteCacheServer::new(&addr, None, Some(temp.path().to_path_buf()));
        let _handle = server.start_background().unwrap();
        std::thread::sleep(Duration::from_millis(50));

        let client: Box<dyn RemoteCacheClient> = Box::new(TcpRemoteCacheClient::new(&addr, None));
        assert_eq!(client.get_artifact("test/artifact").unwrap(), None);

        let payload = b"artifact-binary-payload-data";
        client.put_artifact("test/artifact", payload).unwrap();

        let retrieved = client.get_artifact("test/artifact").unwrap();
        assert_eq!(retrieved, Some(payload.to_vec()));

        server.stop();
    }
}
