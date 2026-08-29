use banana::p2p::{P2PNode, P2PSwarmManager, PeerDescriptor};
use std::net::SocketAddr;
use std::sync::Arc;

#[derive(Clone)]
pub struct BananaMeshCache {
    manager: Arc<P2PSwarmManager>,
}

impl BananaMeshCache {
    pub fn new(node_id: impl Into<String>, bind_addr: SocketAddr) -> Self {
        let node = P2PNode::new(node_id, bind_addr);
        Self {
            manager: Arc::new(P2PSwarmManager::new(node)),
        }
    }

    pub fn store_artifact(&self, key: &str, data: Vec<u8>) {
        self.manager.get_local_node().store_artifact(key, data);
    }

    pub fn get_local_artifact(&self, key: &str) -> Option<Vec<u8>> {
        self.manager.get_local_node().get_artifact(key)
    }

    pub fn register_peer(&self, peer: PeerDescriptor) {
        self.manager.register_peer(peer);
    }

    pub fn find_peer_nodes(&self, key: &str) -> Vec<PeerDescriptor> {
        self.manager.find_peers_with_artifact(key)
    }

    pub fn peer_count(&self) -> usize {
        self.manager.peer_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_banana_mesh_cache_integration() {
        let addr: SocketAddr = "127.0.0.1:9090".parse().unwrap();
        let mesh = BananaMeshCache::new("fish-node-01", addr);
        mesh.store_artifact("pkg-hash-456", b"compiled binary payload".to_vec());

        let data = mesh.get_local_artifact("pkg-hash-456").unwrap();
        assert_eq!(data, b"compiled binary payload");
        assert_eq!(mesh.peer_count(), 0);
    }
}
