use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Permission {
    ReadTarget,
    BuildTarget,
    DeployTarget,
    AdminTarget,
    SignArtifact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    pub name: String,
    pub permissions: HashSet<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityClaims {
    pub subject: String,
    pub email: String,
    pub roles: Vec<String>,
    pub issuer: String,
    pub expires_at: u64,
}

pub struct AccessController {
    roles: HashMap<String, Role>,
    target_rules: Vec<TargetRule>,
}

impl AccessController {
    pub fn new() -> Self {
        let mut controller = Self {
            roles: HashMap::new(),
            target_rules: Vec::new(),
        };

        let mut developer_perms = HashSet::new();
        developer_perms.insert(Permission::ReadTarget);
        developer_perms.insert(Permission::BuildTarget);
        controller.register_role("developer", developer_perms);

        let mut release_perms = HashSet::new();
        release_perms.insert(Permission::ReadTarget);
        release_perms.insert(Permission::BuildTarget);
        release_perms.insert(Permission::DeployTarget);
        release_perms.insert(Permission::SignArtifact);
        controller.register_role("release-manager", release_perms);

        let mut admin_perms = HashSet::new();
        admin_perms.insert(Permission::ReadTarget);
        admin_perms.insert(Permission::BuildTarget);
        admin_perms.insert(Permission::DeployTarget);
        admin_perms.insert(Permission::AdminTarget);
        admin_perms.insert(Permission::SignArtifact);
        controller.register_role("admin", admin_perms);

        controller
    }

    pub fn register_role(&mut self, name: &str, permissions: HashSet<Permission>) {
        self.roles.insert(
            name.to_string(),
            Role {
                name: name.to_string(),
                permissions,
            },
        );
    }

    pub fn check_permission(&self, claims: &IdentityClaims, required: Permission) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if claims.expires_at > 0 && claims.expires_at < now {
            return false;
        }

        for role_name in &claims.roles {
            if let Some(role) = self.roles.get(role_name)
                && role.permissions.contains(&required)
            {
                return true;
            }
        }

        false
    }
}

impl Default for AccessController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rbac_access_control() {
        let ac = AccessController::new();

        let claims = IdentityClaims {
            subject: "user_123".to_string(),
            email: "dev@company.com".to_string(),
            roles: vec!["developer".to_string()],
            issuer: "https://auth.company.com".to_string(),
            expires_at: 9999999999,
        };

        assert!(ac.check_permission(&claims, Permission::BuildTarget));
        assert!(!ac.check_permission(&claims, Permission::AdminTarget));
    }
}

/// Resource-scoped authorization: a permission alone is not enough for
/// sensitive targets — `prod/*` may demand a higher clearance than `dev/*`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TargetRule {
    /// Exact target name, or a prefix pattern ending in `*`.
    pub pattern: String,
    pub min_permission: Permission,
}

impl TargetRule {
    fn matches(&self, target: &str) -> bool {
        if let Some(prefix) = self.pattern.strip_suffix('*') {
            target.starts_with(prefix)
        } else {
            self.pattern == target
        }
    }
}

fn permission_rank(permission: &Permission) -> u8 {
    match permission {
        Permission::ReadTarget => 0,
        Permission::BuildTarget => 1,
        Permission::DeployTarget => 2,
        Permission::AdminTarget => 3,
        // Signing is orthogonal: it ranks with deploy for target gating.
        Permission::SignArtifact => 2,
    }
}

/// Outcome of a resource-scoped authorization check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub allowed: bool,
    pub reason: String,
}

impl AccessController {
    /// Register (or replace) a resource-scoped rule. Rules are evaluated in
    /// registration order and the first matching rule wins.
    pub fn register_target_rule(&mut self, rule: TargetRule) {
        self.target_rules.retain(|r| r.pattern != rule.pattern);
        self.target_rules.push(rule);
    }

    /// Authorize `subject` to perform `required` on `target`.
    ///
    /// Both gates must pass: the identity must hold `required` through one of
    /// its roles, and every matching target rule must be satisfied by a role
    /// whose rank reaches the rule's minimum permission.
    pub fn authorize_target(
        &self,
        claims: &IdentityClaims,
        target: &str,
        required: Permission,
    ) -> Decision {
        if !self.check_permission(claims, required.clone()) {
            return Decision {
                allowed: false,
                reason: format!(
                    "identity `{}` lacks permission {required:?}",
                    claims.subject
                ),
            };
        }

        let held_rank = claims
            .roles
            .iter()
            .filter_map(|name| self.roles.get(name))
            .flat_map(|role| role.permissions.iter())
            .map(permission_rank)
            .max()
            .unwrap_or(0);

        for rule in &self.target_rules {
            if rule.matches(target) && permission_rank(&rule.min_permission) > held_rank {
                return Decision {
                    allowed: false,
                    reason: format!(
                        "target `{target}` requires {:?} but identity `{}` tops out lower",
                        rule.min_permission, claims.subject
                    ),
                };
            }
        }

        Decision {
            allowed: true,
            reason: format!("identity `{}` authorized for `{target}`", claims.subject),
        }
    }
}

/// One append-only authorization decision, written as JSON Lines so external
/// systems can tail the file without locking coordination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp_unix_secs: u64,
    pub subject: String,
    pub action: String,
    pub target: String,
    pub allowed: bool,
}

pub struct AuditLog {
    path: std::path::PathBuf,
}

impl AuditLog {
    pub fn new(path: impl Into<std::path::PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn record(&self, entry: &AuditEntry) -> std::io::Result<()> {
        use std::io::Write;
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut line = serde_json::to_string(entry)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        line.push('\n');
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        file.write_all(line.as_bytes())
    }

    pub fn read_all(&self) -> std::io::Result<Vec<AuditEntry>> {
        if !self.path.exists() {
            return Ok(Vec::new());
        }
        std::fs::read_to_string(&self.path)?
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                serde_json::from_str(line)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            })
            .collect()
    }
}

#[cfg(test)]
mod scoped_authorization_tests {
    use super::*;

    fn claims(roles: &[&str]) -> IdentityClaims {
        IdentityClaims {
            subject: "user_123".to_string(),
            email: "dev@company.com".to_string(),
            roles: roles.iter().map(|r| r.to_string()).collect(),
            issuer: "https://auth.company.com".to_string(),
            expires_at: 9999999999,
        }
    }

    #[test]
    fn test_prod_targets_demand_higher_clearance() {
        let mut ac = AccessController::new();
        ac.register_target_rule(TargetRule {
            pattern: "prod/*".to_string(),
            min_permission: Permission::AdminTarget,
        });

        let dev = claims(&["developer"]);
        assert!(
            !ac.authorize_target(&dev, "prod/api", Permission::BuildTarget)
                .allowed
        );
        assert!(
            ac.authorize_target(&dev, "dev/api", Permission::BuildTarget)
                .allowed
        );

        let admin = claims(&["admin"]);
        assert!(
            ac.authorize_target(&admin, "prod/api", Permission::BuildTarget)
                .allowed
        );
    }

    #[test]
    fn test_exact_pattern_and_first_rule_wins() {
        let mut ac = AccessController::new();
        ac.register_target_rule(TargetRule {
            pattern: "infra/golden".to_string(),
            min_permission: Permission::DeployTarget,
        });
        ac.register_target_rule(TargetRule {
            pattern: "infra/*".to_string(),
            min_permission: Permission::ReadTarget,
        });

        let dev = claims(&["developer"]);
        assert!(
            !ac.authorize_target(&dev, "infra/golden", Permission::BuildTarget)
                .allowed
        );
        assert!(
            ac.authorize_target(&dev, "infra/other", Permission::BuildTarget)
                .allowed
        );
    }

    #[test]
    fn test_audit_log_appends_and_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        let log = AuditLog::new(dir.path().join("audit").join("decisions.jsonl"));

        for allowed in [true, false] {
            log.record(&AuditEntry {
                timestamp_unix_secs: 1_700_000_000,
                subject: "user_123".to_string(),
                action: "BuildTarget".to_string(),
                target: "prod/api".to_string(),
                allowed,
            })
            .unwrap();
        }

        let entries = log.read_all().unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].allowed);
        assert!(!entries[1].allowed);
    }
}
