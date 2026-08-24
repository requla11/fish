//! Cross-region cache replication topology.
//!
//! Builds on the LAN peer registry to add region-awareness, replication
//! policies, and conflict resolution for geo-distributed CAS artifacts.
//! Content addressing (BLAKE3) eliminates true conflicts; this module
//! handles *catalog* consistency and transfer budgeting across regions.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;

/// A logical deployment region (e.g. `us-east-1`, `ap-south-1`).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RegionId(pub String);

/// Replication policy controlling how artifacts propagate between regions.
#[derive(Debug, Clone)]
pub struct ReplicationPolicy {
    /// Maximum number of peer regions each artifact replicates to.
    pub max_replica_regions: usize,
    /// Minimum number of healthy replicas before a region stops requesting.
    pub min_healthy_replicas: usize,
    /// TTL in seconds after which a region's catalog entry is considered
    /// stale and refreshed from the origin.
    pub catalog_ttl_secs: u64,
}

impl Default for ReplicationPolicy {
    fn default() -> Self {
        Self {
            max_replica_regions: 3,
            min_healthy_replicas: 1,
            catalog_ttl_secs: 3600,
        }
    }
}

/// A region node participating in the replication mesh.
#[derive(Debug, Clone)]
pub struct RegionNode {
    pub region: RegionId,
    pub endpoint: String,
    /// Artifacts this region currently holds.
    pub catalog: std::collections::HashSet<String>,
    pub last_sync_unix_secs: u64,
    pub is_healthy: bool,
}

/// Tracks which regions hold which artifacts and decides when/where to
/// replicate. Content-addressed storage means the artifact payload itself is
/// immutable — only the *catalog* needs consensus.
pub struct ReplicationTopology {
    local_region: RegionId,
    policy: ReplicationPolicy,
    regions: Arc<RwLock<HashMap<RegionId, RegionNode>>>,
}

impl ReplicationTopology {
    pub fn new(local_region: RegionId, policy: ReplicationPolicy) -> Self {
        Self {
            local_region,
            policy,
            regions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a remote region endpoint.
    pub fn register_region(&self, node: RegionNode) {
        self.regions
            .write()
            .unwrap()
            .insert(node.region.clone(), node);
    }

    /// Mark a region as unhealthy so it's excluded from replication targets.
    pub fn mark_unhealthy(&self, region: &RegionId) {
        if let Some(node) = self.regions.write().unwrap().get_mut(region) {
            node.is_healthy = false;
        }
    }

    pub fn mark_healthy(&self, region: &RegionId) {
        if let Some(node) = self.regions.write().unwrap().get_mut(region) {
            node.is_healthy = true;
        }
    }

    /// Record that `artifact_hash` is now available in `region`.
    pub fn announce_artifact(&self, region: &RegionId, artifact_hash: &str) {
        if let Some(node) = self.regions.write().unwrap().get_mut(region) {
            node.catalog.insert(artifact_hash.to_string());
            node.last_sync_unix_secs = now_secs();
        }
    }

    /// Determine which regions should receive a newly cached artifact.
    ///
    /// Selects up to `max_replica_regions` healthy regions that don't already
    /// have it, prioritising those with fewer replicas (spread load).
    pub fn select_replication_targets(&self, artifact_hash: &str) -> Vec<RegionId> {
        let map = self.regions.read().unwrap();
        let mut candidates: Vec<(&RegionId, &RegionNode)> = map
            .iter()
            .filter(|(id, node)| {
                **id != self.local_region
                    && node.is_healthy
                    && !node.catalog.contains(artifact_hash)
            })
            .collect();

        // Sort by catalog size ascending — regions with fewer artifacts get
        // priority so the mesh stays balanced.
        candidates.sort_by_key(|(_, node)| node.catalog.len());

        candidates
            .into_iter()
            .take(self.policy.max_replica_regions)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Find the closest healthy region holding `artifact_hash`.
    pub fn locate_artifact(&self, artifact_hash: &str) -> Option<RegionId> {
        let map = self.regions.read().unwrap();
        map.iter()
            .filter(|(id, node)| {
                **id != self.local_region && node.is_healthy && node.catalog.contains(artifact_hash)
            })
            .map(|(id, _)| id.clone())
            .next()
    }

    /// Count healthy replicas of an artifact across all known regions.
    pub fn replica_count(&self, artifact_hash: &str) -> usize {
        let map = self.regions.read().unwrap();
        map.values()
            .filter(|node| node.is_healthy && node.catalog.contains(artifact_hash))
            .count()
    }

    /// Whether more replicas are needed per policy.
    pub fn needs_replication(&self, artifact_hash: &str) -> bool {
        self.replica_count(artifact_hash) < self.policy.min_healthy_replicas
    }

    /// Remove stale catalog entries whose last sync exceeds TTL.
    pub fn evict_stale_entries(&self) {
        let cutoff = now_secs().saturating_sub(self.policy.catalog_ttl_secs);
        let mut map = self.regions.write().unwrap();
        for node in map.values_mut() {
            if node.last_sync_unix_secs < cutoff {
                node.catalog.clear();
            }
        }
    }

    pub fn local_region(&self) -> &RegionId {
        &self.local_region
    }

    pub fn region_count(&self) -> usize {
        self.regions.read().unwrap().len()
    }
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(region: &str, endpoint: &str) -> RegionNode {
        RegionNode {
            region: RegionId(region.to_string()),
            endpoint: endpoint.to_string(),
            catalog: std::collections::HashSet::new(),
            last_sync_unix_secs: now_secs(),
            is_healthy: true,
        }
    }

    #[test]
    fn test_select_replication_targets_excludes_healthy_holders() {
        let topo =
            ReplicationTopology::new(RegionId("us-east-1".into()), ReplicationPolicy::default());
        topo.register_region(node("eu-west-1", "https://eu1.fish.internal"));
        topo.register_region(node("ap-south-1", "https://aps1.fish.internal"));
        topo.register_region(node("sa-east-1", "https://sae1.fish.internal"));

        // No one has the artifact yet → all 3 are targets.
        let targets = topo.select_replication_targets("abc123");
        assert_eq!(targets.len(), 3);

        // After announcing in eu-west-1, only 2 remain.
        topo.announce_artifact(&RegionId("eu-west-1".into()), "abc123");
        let targets = topo.select_replication_targets("abc123");
        assert_eq!(targets.len(), 2);
        assert!(!targets.contains(&RegionId("eu-west-1".into())));
    }

    #[test]
    fn test_replica_count_and_needs_replication() {
        let policy = ReplicationPolicy {
            min_healthy_replicas: 2,
            ..Default::default()
        };
        let topo = ReplicationTopology::new(RegionId("local".into()), policy);
        topo.register_region(node("r1", "ep1"));
        topo.register_region(node("r2", "ep2"));

        assert!(topo.needs_replication("artifact_x"));
        topo.announce_artifact(&RegionId("r1".into()), "artifact_x");
        assert!(topo.needs_replication("artifact_x"), "only 1 of min 2");
        topo.announce_artifact(&RegionId("r2".into()), "artifact_x");
        assert!(!topo.needs_replication("artifact_x"), "met min 2");
    }

    #[test]
    fn test_unhealthy_regions_are_skipped() {
        let topo = ReplicationTopology::new(RegionId("local".into()), ReplicationPolicy::default());
        topo.register_region(node("healthy", "ep1"));
        topo.register_region(node("dead", "ep2"));
        topo.mark_unhealthy(&RegionId("dead".into()));

        assert_eq!(
            topo.locate_artifact("some_artifact"),
            None,
            "unhealthy regions excluded from lookup"
        );

        topo.announce_artifact(&RegionId("dead".into()), "some_artifact");
        assert_eq!(
            topo.locate_artifact("some_artifact"),
            None,
            "still excluded after announce on unhealthy node"
        );
        topo.announce_artifact(&RegionId("healthy".into()), "some_artifact");
        assert_eq!(topo.locate_artifact("some_artifact").unwrap().0, "healthy");
    }

    #[test]
    fn test_evict_stale_entries_clears_old_catalog() {
        let policy = ReplicationPolicy {
            catalog_ttl_secs: 60,
            ..Default::default()
        };
        let topo = ReplicationTopology::new(RegionId("l".into()), policy);
        let mut n = node("old", "ep");
        n.last_sync_unix_secs = 100; // very old
        topo.register_region(n);

        topo.announce_artifact(&RegionId("old".into()), "stale_artifact");
        assert_eq!(topo.replica_count("stale_artifact"), 1);

        // Force the region's sync timestamp back into the past.
        if let Some(n) = topo
            .regions
            .write()
            .unwrap()
            .get_mut(&RegionId("old".into()))
        {
            n.last_sync_unix_secs = 100;
        }

        topo.evict_stale_entries();
        assert_eq!(
            topo.replica_count("stale_artifact"),
            0,
            "stale entry evicted"
        );
    }
}
