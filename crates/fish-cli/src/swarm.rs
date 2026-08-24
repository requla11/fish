#![allow(dead_code)]

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

pub const SWARM_DISCOVERY_PORT: u16 = 7890;

/// Environment variable holding the shared swarm secret. When set, every
/// announcement carries a Blake3 digest of the secret; peers with a different
/// (or no) secret silently ignore the message. When unset, discovery runs in
/// trusted-LAN mode with an empty token.
const SWARM_TOKEN_ENV: &str = "FISH_SWARM_TOKEN";

#[derive(Debug, Clone)]
pub struct SwarmPeer {
    pub peer_id: String,
    pub address: SocketAddr,
    pub compute_port: Option<u16>,
    pub concurrency: usize,
    pub last_seen: Instant,
}

/// Derives the authentication token carried by announcements of `kind`
/// ("announce" or "compute"). Returns an empty token when no shared secret
/// is configured, keeping discovery usable on trusted networks.
fn auth_token(kind: &str) -> String {
    let Ok(secret) = std::env::var(SWARM_TOKEN_ENV) else {
        return String::new();
    };
    if secret.is_empty() {
        return String::new();
    }
    let mut hasher = blake3::Hasher::new();
    hasher.update(b"fish-swarm-v1");
    hasher.update(kind.as_bytes());
    hasher.update(secret.as_bytes());
    hasher.finalize().to_hex()[..16].to_string()
}

/// Validates an `FISH_SWARM_ANNOUNCE:{token}:{peer_id}:{port}` message and
/// returns `(peer_id, port)` when the token matches.
fn parse_announce<'a>(message: &'a str, expected_token: &str) -> Option<(&'a str, u16)> {
    let payload = message.strip_prefix("FISH_SWARM_ANNOUNCE:")?;
    let parts: Vec<&str> = payload.split(':').collect();
    if parts.len() != 3 || parts[0] != expected_token {
        return None;
    }
    Some((parts[1], parts[2].parse().ok()?))
}

/// Validates an `FISH_SWARM_COMPUTE:{token}:{peer_id}:{port}:{concurrency}`
/// message and returns `(peer_id, port, concurrency)` when the token matches.
fn parse_compute<'a>(message: &'a str, expected_token: &str) -> Option<(&'a str, u16, usize)> {
    let payload = message.strip_prefix("FISH_SWARM_COMPUTE:")?;
    let parts: Vec<&str> = payload.split(':').collect();
    if parts.len() != 4 || parts[0] != expected_token {
        return None;
    }
    let concurrency = parts[3].parse::<usize>().unwrap_or(4);
    Some((parts[1], parts[2].parse().ok()?, concurrency))
}

#[derive(Debug, Clone)]
pub struct SwarmCache {
    peer_id: String,
    peers: Arc<RwLock<HashMap<String, SwarmPeer>>>,
    socket: Option<Arc<UdpSocket>>,
    enabled: bool,
}

impl SwarmCache {
    pub fn new(enabled: bool) -> Self {
        // The system clock may run behind the epoch on misconfigured machines;
        // fall back to a constant instead of panicking during startup.
        let uptime_millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let peer_id = format!("peer_{uptime_millis:x}");

        let socket = if enabled {
            let sock = UdpSocket::bind("0.0.0.0:0").ok();
            if let Some(ref s) = sock {
                let _ = s.set_broadcast(true);
                let _ = s.set_read_timeout(Some(Duration::from_millis(50)));
            }
            sock.map(Arc::new)
        } else {
            None
        };

        Self {
            peer_id,
            peers: Arc::new(RwLock::new(HashMap::new())),
            socket,
            enabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn broadcast_presence(&self, service_port: u16) -> std::io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if let Some(ref socket) = self.socket {
            let message = format!(
                "FISH_SWARM_ANNOUNCE:{}:{}:{}",
                auth_token("announce"),
                self.peer_id,
                service_port
            );
            let broadcast_addr = SocketAddr::from(([255, 255, 255, 255], SWARM_DISCOVERY_PORT));
            socket.send_to(message.as_bytes(), broadcast_addr)?;
        }

        Ok(())
    }

    pub fn broadcast_compute_worker(
        &self,
        worker_port: u16,
        concurrency: usize,
    ) -> std::io::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        if let Some(ref socket) = self.socket {
            let message = format!(
                "FISH_SWARM_COMPUTE:{}:{}:{}:{}",
                auth_token("compute"),
                self.peer_id,
                worker_port,
                concurrency
            );
            let broadcast_addr = SocketAddr::from(([255, 255, 255, 255], SWARM_DISCOVERY_PORT));
            socket.send_to(message.as_bytes(), broadcast_addr)?;
        }

        Ok(())
    }

    pub fn poll_peers(&self) {
        if !self.enabled {
            return;
        }

        let announce_token = auth_token("announce");
        let compute_token = auth_token("compute");

        if let Some(ref socket) = self.socket {
            let mut buf = [0u8; 1024];
            while let Ok((amt, src)) = socket.recv_from(&mut buf) {
                let Ok(msg) = std::str::from_utf8(&buf[..amt]) else {
                    continue;
                };

                if let Some((peer_id, port)) = parse_announce(msg, &announce_token) {
                    if peer_id == self.peer_id {
                        continue;
                    }
                    let mut peer_addr = src;
                    peer_addr.set_port(port);
                    if let Ok(mut peers) = self.peers.write() {
                        peers.insert(
                            peer_id.to_string(),
                            SwarmPeer {
                                peer_id: peer_id.to_string(),
                                address: peer_addr,
                                compute_port: None,
                                concurrency: 0,
                                last_seen: Instant::now(),
                            },
                        );
                    }
                } else if let Some((peer_id, compute_port, concurrency)) =
                    parse_compute(msg, &compute_token)
                {
                    if peer_id == self.peer_id {
                        continue;
                    }
                    let mut peer_addr = src;
                    peer_addr.set_port(compute_port);
                    if let Ok(mut peers) = self.peers.write() {
                        peers.insert(
                            peer_id.to_string(),
                            SwarmPeer {
                                peer_id: peer_id.to_string(),
                                address: peer_addr,
                                compute_port: Some(compute_port),
                                concurrency,
                                last_seen: Instant::now(),
                            },
                        );
                    }
                }
            }
        }
    }

    pub fn active_peer_count(&self) -> usize {
        self.poll_peers();
        if let Ok(peers) = self.peers.read() {
            let now = Instant::now();
            peers
                .values()
                .filter(|p| now.duration_since(p.last_seen) < Duration::from_secs(60))
                .count()
        } else {
            0
        }
    }

    pub fn discovered_compute_endpoints(&self) -> Vec<String> {
        self.poll_peers();
        if let Ok(peers) = self.peers.read() {
            let now = Instant::now();
            peers
                .values()
                .filter(|p| {
                    p.compute_port.is_some()
                        && now.duration_since(p.last_seen) < Duration::from_secs(60)
                })
                .map(|p| format!("http://{}", p.address))
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn list_peers(&self) -> Vec<SwarmPeer> {
        self.poll_peers();
        if let Ok(peers) = self.peers.read() {
            peers.values().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swarm_cache_disabled() {
        let swarm = SwarmCache::new(false);
        assert!(!swarm.is_enabled());
        assert_eq!(swarm.active_peer_count(), 0);
        assert!(swarm.list_peers().is_empty());
        assert!(swarm.discovered_compute_endpoints().is_empty());
    }

    #[test]
    fn test_swarm_cache_enabled_initial_state() {
        let swarm = SwarmCache::new(true);
        assert!(swarm.is_enabled());
        assert_eq!(swarm.active_peer_count(), 0);
        let _ = swarm.broadcast_presence(7890);
        let _ = swarm.broadcast_compute_worker(7891, 8);
    }

    #[test]
    fn test_parse_announce_accepts_valid_and_rejects_bad_tokens() {
        let token = "abcd1234abcd1234";

        let msg = format!("FISH_SWARM_ANNOUNCE:{token}:peer_abc:8080");
        assert_eq!(
            parse_announce(&msg, token),
            Some(("peer_abc", 8080)),
            "matching token must be accepted"
        );

        assert_eq!(
            parse_announce(&msg, "ffffffffffffffff"),
            None,
            "wrong token must be rejected"
        );

        let trusted_lan = "FISH_SWARM_ANNOUNCE::peer_abc:8080";
        assert_eq!(
            parse_announce(trusted_lan, ""),
            Some(("peer_abc", 8080)),
            "empty token (trusted-LAN mode) must be accepted"
        );

        let malformed = format!("FISH_SWARM_ANNOUNCE:{token}:peer_abc:notaport");
        assert_eq!(parse_announce(&malformed, token), None);

        assert_eq!(parse_announce("TOTALLY_DIFFERENT:x", token), None);
    }

    #[test]
    fn test_parse_compute_round_trip() {
        let token = "1234123412341234";

        let msg = format!("FISH_SWARM_COMPUTE:{token}:peer_xyz:9091:6");
        assert_eq!(
            parse_compute(&msg, token),
            Some(("peer_xyz", 9091, 6)),
            "valid compute announcement must round-trip"
        );

        let default_concurrency = format!("FISH_SWARM_COMPUTE:{token}:peer_xyz:9091:notanumber");
        assert_eq!(
            parse_compute(&default_concurrency, token),
            Some(("peer_xyz", 9091, 4)),
            "unparsable concurrency falls back to 4"
        );

        assert_eq!(
            parse_compute(&msg, "0000000000000000"),
            None,
            "wrong token must be rejected"
        );
    }
}
