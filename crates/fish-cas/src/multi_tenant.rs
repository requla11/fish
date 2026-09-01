//! Multi-tenant cache isolation with per-namespace quotas.
//!
//! Wraps [`CasStorage`] to add tenant namespacing: every key is prefixed
//! with a tenant identifier so different teams cannot read each other's
//! artifacts. Per-tenant byte quotas are enforced at write time by
//! tracking cumulative usage from metadata.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Per-tenant quota configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantQuota {
    /// Maximum total bytes this tenant may store.
    pub max_bytes: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TenantQuotas {
    pub tenants: HashMap<String, TenantQuota>,
    /// Default limit for unlisted tenants; `None` means unlimited.
    pub default_max_bytes: Option<u64>,
}

impl TenantQuotas {
    pub fn max_bytes_for(&self, tenant: &str) -> Option<u64> {
        self.tenants
            .get(tenant)
            .map(|q| q.max_bytes)
            .or(self.default_max_bytes)
    }
}

/// Tracks per-tenant storage usage for quota enforcement.
#[derive(Debug, Default)]
pub struct TenantUsageTracker {
    usage: std::sync::Mutex<HashMap<String, u64>>,
}

impl TenantUsageTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a write of `bytes` for `tenant`. Returns `Err(current_usage)`
    /// if the write would exceed the quota.
    pub fn record_write(
        &self,
        tenant: &str,
        bytes: u64,
        quotas: &TenantQuotas,
    ) -> Result<u64, u64> {
        let mut guard = self.usage.lock().unwrap();
        let current = guard.entry(tenant.to_string()).or_insert(0);
        let projected = *current + bytes;
        if let Some(max) = quotas.max_bytes_for(tenant) && projected > max {
            return Err(*current);
        }
        *current = projected;
        Ok(projected)
    }

    pub fn usage_for(&self, tenant: &str) -> u64 {
        self.usage.lock().unwrap().get(tenant).copied().unwrap_or(0)
    }
}

/// Build a namespaced key for a tenant: `{tenant}:{original_key}`.
pub fn tenant_key(tenant: &str, key: &str) -> String {
    format!("{tenant}:{key}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tenant_key_namespacing() {
        assert_eq!(tenant_key("team-a", "abc123"), "team-a:abc123");
        assert_ne!(
            tenant_key("team-a", "shared_key"),
            tenant_key("team-b", "shared_key")
        );
    }

    #[test]
    fn test_quota_enforcement() {
        let quotas = TenantQuotas {
            tenants: HashMap::from([("team-a".to_string(), TenantQuota { max_bytes: 100 })]),
            default_max_bytes: Some(50),
        };
        let tracker = TenantUsageTracker::new();

        // team-a writes 60 bytes (within 100 limit)
        assert!(tracker.record_write("team-a", 60, &quotas).is_ok());
        // team-a writes 50 more → total 110 > 100 → denied
        assert!(tracker.record_write("team-a", 50, &quotas).is_err());

        // team-b has default limit of 50, writes 40 → OK
        assert!(tracker.record_write("team-b", 40, &quotas).is_ok());
        // team-b writes 20 more → total 60 > 50 → denied
        assert!(tracker.record_write("team-b", 20, &quotas).is_err());
    }

    #[test]
    fn test_unlimited_when_no_default_and_no_tenant_entry() {
        let quotas = TenantQuotas::default();
        let tracker = TenantUsageTracker::new();
        // No quota configured → always succeeds
        assert!(
            tracker
                .record_write("anyone", u64::MAX / 2, &quotas)
                .is_ok()
        );
    }

    #[test]
    fn test_usage_tracking() {
        let quotas = TenantQuotas::default();
        let tracker = TenantUsageTracker::new();
        tracker.record_write("t1", 10, &quotas).unwrap();
        tracker.record_write("t1", 20, &quotas).unwrap();
        assert_eq!(tracker.usage_for("t1"), 30);
        assert_eq!(tracker.usage_for("nonexistent"), 0);
    }
}
