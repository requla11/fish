use base64::{Engine as _, engine::general_purpose};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
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
    #[serde(default)]
    pub audience: Option<String>,
    #[serde(default)]
    pub not_before: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetRule {
    pub pattern: String,
    pub min_permission: Permission,
}

impl TargetRule {
    pub fn matches(&self, target: &str) -> bool {
        if let Some(prefix) = self.pattern.strip_suffix('*') {
            target.starts_with(prefix)
        } else {
            self.pattern == target
        }
    }
}

pub fn permission_rank(permission: &Permission) -> u8 {
    match permission {
        Permission::ReadTarget => 0,
        Permission::BuildTarget => 1,
        Permission::DeployTarget => 2,
        Permission::AdminTarget => 3,
        Permission::SignArtifact => 2,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub allowed: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    #[serde(default)]
    pub allowed_issuers: Vec<String>,
    #[serde(default)]
    pub required_audience: Option<String>,
    #[serde(default)]
    pub trusted_ed25519_keys: HashMap<String, String>,
    #[serde(default)]
    pub hmac_secrets: HashMap<String, String>,
    #[serde(default = "default_role_claim")]
    pub role_claim: String,
    #[serde(default)]
    pub role_mappings: HashMap<String, String>,
}

fn default_role_claim() -> String {
    "roles".to_string()
}

impl Default for OidcConfig {
    fn default() -> Self {
        Self {
            allowed_issuers: Vec::new(),
            required_audience: None,
            trusted_ed25519_keys: HashMap::new(),
            hmac_secrets: HashMap::new(),
            role_claim: default_role_claim(),
            role_mappings: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityPolicy {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub oidc: Option<OidcConfig>,
    #[serde(default)]
    pub roles: HashMap<String, HashSet<Permission>>,
    #[serde(default)]
    pub target_rules: Vec<TargetRule>,
    #[serde(default)]
    pub audit_log_path: Option<PathBuf>,
}

pub struct AccessController {
    roles: HashMap<String, Role>,
    target_rules: Vec<TargetRule>,
    oidc_config: Option<OidcConfig>,
    audit_log: Option<AuditLog>,
}

impl AccessController {
    pub fn new() -> Self {
        let mut controller = Self {
            roles: HashMap::new(),
            target_rules: Vec::new(),
            oidc_config: None,
            audit_log: None,
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

    pub fn from_policy(policy: &SecurityPolicy) -> Self {
        let mut controller = Self::new();
        for (name, perms) in &policy.roles {
            controller.register_role(name, perms.clone());
        }
        for rule in &policy.target_rules {
            controller.register_target_rule(rule.clone());
        }
        controller.oidc_config = policy.oidc.clone();
        if let Some(audit_path) = &policy.audit_log_path {
            controller.audit_log = Some(AuditLog::new(audit_path));
        }
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

    pub fn register_target_rule(&mut self, rule: TargetRule) {
        self.target_rules.retain(|r| r.pattern != rule.pattern);
        self.target_rules.push(rule);
    }

    pub fn set_oidc_config(&mut self, config: OidcConfig) {
        self.oidc_config = Some(config);
    }

    pub fn set_audit_log(&mut self, log: AuditLog) {
        self.audit_log = Some(log);
    }

    pub fn check_permission(&self, claims: &IdentityClaims, required: Permission) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if claims.expires_at > 0 && claims.expires_at < now {
            return false;
        }

        if let Some(nbf) = claims.not_before
            && nbf > now
        {
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

    pub fn authorize_target(
        &self,
        claims: &IdentityClaims,
        target: &str,
        required: Permission,
    ) -> Decision {
        let decision = self.evaluate_target(claims, target, required.clone());
        if let Some(audit) = &self.audit_log {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let entry = AuditEntry {
                timestamp_unix_secs: now,
                subject: claims.subject.clone(),
                action: format!("{required:?}"),
                target: target.to_string(),
                allowed: decision.allowed,
            };
            let _ = audit.record(&entry);
        }
        decision
    }

    fn evaluate_target(
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

    pub fn authenticate_token(&self, raw_token: &str) -> Result<IdentityClaims, String> {
        let config = self
            .oidc_config
            .as_ref()
            .ok_or_else(|| "OIDC is not configured".to_string())?;
        validate_oidc_jwt(raw_token, config)
    }
}

impl Default for AccessController {
    fn default() -> Self {
        Self::new()
    }
}

fn decode_b64url(input: &str) -> Result<Vec<u8>, String> {
    let unpadded = input.trim_end_matches('=');
    general_purpose::URL_SAFE_NO_PAD
        .decode(unpadded)
        .or_else(|_| general_purpose::STANDARD_NO_PAD.decode(unpadded))
        .or_else(|_| general_purpose::STANDARD.decode(input))
        .map_err(|e| format!("Base64 decoding failed: {e}"))
}

pub fn compute_hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    let mut key_block = [0u8; 64];
    if key.len() > 64 {
        let hashed = Sha256::digest(key);
        key_block[..32].copy_from_slice(&hashed);
    } else {
        key_block[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0u8; 64];
    let mut opad = [0u8; 64];
    for i in 0..64 {
        ipad[i] = key_block[i] ^ 0x36;
        opad[i] = key_block[i] ^ 0x5c;
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_hash = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    outer.finalize().into()
}

pub fn create_signed_jwt_hmac(
    claims: &serde_json::Value,
    secret: &[u8],
    key_id: Option<&str>,
) -> Result<String, String> {
    let mut header_map = serde_json::Map::new();
    header_map.insert("alg".to_string(), serde_json::json!("HS256"));
    header_map.insert("typ".to_string(), serde_json::json!("JWT"));
    if let Some(kid) = key_id {
        header_map.insert("kid".to_string(), serde_json::json!(kid));
    }
    let header_json = serde_json::to_string(&header_map)
        .map_err(|e| format!("failed to serialize header: {e}"))?;
    let claims_json =
        serde_json::to_string(claims).map_err(|e| format!("failed to serialize claims: {e}"))?;

    let header_b64 = general_purpose::URL_SAFE_NO_PAD.encode(header_json.as_bytes());
    let claims_b64 = general_purpose::URL_SAFE_NO_PAD.encode(claims_json.as_bytes());
    let signing_input = format!("{header_b64}.{claims_b64}");

    let signature = compute_hmac_sha256(secret, signing_input.as_bytes());
    let sig_b64 = general_purpose::URL_SAFE_NO_PAD.encode(signature);

    Ok(format!("{signing_input}.{sig_b64}"))
}

pub fn create_signed_jwt_ed25519(
    claims: &serde_json::Value,
    signing_key: &ed25519_dalek::SigningKey,
    key_id: Option<&str>,
) -> Result<String, String> {
    use ed25519_dalek::Signer;

    let mut header_map = serde_json::Map::new();
    header_map.insert("alg".to_string(), serde_json::json!("EdDSA"));
    header_map.insert("typ".to_string(), serde_json::json!("JWT"));
    if let Some(kid) = key_id {
        header_map.insert("kid".to_string(), serde_json::json!(kid));
    }
    let header_json = serde_json::to_string(&header_map)
        .map_err(|e| format!("failed to serialize header: {e}"))?;
    let claims_json =
        serde_json::to_string(claims).map_err(|e| format!("failed to serialize claims: {e}"))?;

    let header_b64 = general_purpose::URL_SAFE_NO_PAD.encode(header_json.as_bytes());
    let claims_b64 = general_purpose::URL_SAFE_NO_PAD.encode(claims_json.as_bytes());
    let signing_input = format!("{header_b64}.{claims_b64}");

    let signature = signing_key.sign(signing_input.as_bytes());
    let sig_b64 = general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes());

    Ok(format!("{signing_input}.{sig_b64}"))
}

pub fn validate_oidc_jwt(raw_token: &str, config: &OidcConfig) -> Result<IdentityClaims, String> {
    let parts: Vec<&str> = raw_token.trim().split('.').collect();
    if parts.len() != 3 {
        return Err("invalid JWT format: token must contain 3 dot-separated parts".to_string());
    }

    let header_raw = decode_b64url(parts[0])?;
    let header_val: serde_json::Value =
        serde_json::from_slice(&header_raw).map_err(|e| format!("invalid JWT header json: {e}"))?;

    let alg = header_val
        .get("alg")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing alg in JWT header".to_string())?;

    let kid = header_val.get("kid").and_then(|v| v.as_str());

    let payload_raw = decode_b64url(parts[1])?;
    let payload_val: serde_json::Value = serde_json::from_slice(&payload_raw)
        .map_err(|e| format!("invalid JWT payload json: {e}"))?;

    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let signature_bytes = decode_b64url(parts[2])?;

    match alg {
        "HS256" => {
            let secret_str = if let Some(k) = kid {
                config
                    .hmac_secrets
                    .get(k)
                    .ok_or_else(|| format!("unknown key id `{k}` for HS256 verification"))?
            } else if let Some((_, s)) = config.hmac_secrets.iter().next() {
                s
            } else {
                return Err("no HMAC secrets configured in OIDC config".to_string());
            };

            let expected_sig = compute_hmac_sha256(secret_str.as_bytes(), signing_input.as_bytes());
            if signature_bytes.len() != expected_sig.len()
                || signature_bytes.as_slice() != expected_sig.as_slice()
            {
                return Err("HMAC signature verification failed".to_string());
            }
        }
        "EdDSA" => {
            let key_str = if let Some(k) = kid {
                config
                    .trusted_ed25519_keys
                    .get(k)
                    .ok_or_else(|| format!("unknown key id `{k}` for EdDSA verification"))?
            } else if let Some((_, k)) = config.trusted_ed25519_keys.iter().next() {
                k
            } else {
                return Err("no trusted Ed25519 keys configured in OIDC config".to_string());
            };

            let key_bytes: [u8; 32] = decode_b64url(key_str)?
                .try_into()
                .map_err(|_| "invalid 32-byte Ed25519 public key".to_string())?;
            let verifying_key = VerifyingKey::from_bytes(&key_bytes)
                .map_err(|e| format!("invalid verifying key: {e}"))?;
            let sig_arr: [u8; 64] = signature_bytes
                .try_into()
                .map_err(|_| "invalid 64-byte Ed25519 signature".to_string())?;
            let signature = Signature::from_bytes(&sig_arr);
            verifying_key
                .verify(signing_input.as_bytes(), &signature)
                .map_err(|e| format!("Ed25519 signature verification failed: {e}"))?;
        }
        other => return Err(format!("unsupported JWT alg `{other}`")),
    }

    let subject = payload_val
        .get("sub")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "missing `sub` claim in JWT payload".to_string())?
        .to_string();

    let issuer = payload_val
        .get("iss")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if !config.allowed_issuers.is_empty() && !config.allowed_issuers.contains(&issuer) {
        return Err(format!("untrusted issuer `{issuer}`"));
    }

    if let Some(req_aud) = &config.required_audience {
        let aud_matches = match payload_val.get("aud") {
            Some(serde_json::Value::String(s)) => s == req_aud,
            Some(serde_json::Value::Array(arr)) => {
                arr.iter().any(|v| v.as_str() == Some(req_aud.as_str()))
            }
            _ => false,
        };
        if !aud_matches {
            return Err(format!(
                "audience mismatch: required `{req_aud}`, got {:?}",
                payload_val.get("aud")
            ));
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let expires_at = payload_val.get("exp").and_then(|v| v.as_u64()).unwrap_or(0);

    if expires_at > 0 && expires_at < now {
        return Err(format!(
            "JWT token expired at timestamp {expires_at} (current time {now})"
        ));
    }

    let not_before = payload_val.get("nbf").and_then(|v| v.as_u64());
    if let Some(nbf) = not_before
        && nbf > now
    {
        return Err(format!(
            "JWT token not active before timestamp {nbf} (current time {now})"
        ));
    }

    let email = payload_val
        .get("email")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut roles = Vec::new();
    if let Some(raw_roles) = payload_val.get(&config.role_claim) {
        match raw_roles {
            serde_json::Value::Array(arr) => {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        let mapped = config.role_mappings.get(s).map(String::as_str).unwrap_or(s);
                        roles.push(mapped.to_string());
                    }
                }
            }
            serde_json::Value::String(s) => {
                let mapped = config.role_mappings.get(s).map(String::as_str).unwrap_or(s);
                roles.push(mapped.to_string());
            }
            _ => {}
        }
    }

    if roles.is_empty() {
        roles.push("developer".to_string());
    }

    Ok(IdentityClaims {
        subject,
        email,
        roles,
        issuer,
        expires_at,
        audience: config.required_audience.clone(),
        not_before,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp_unix_secs: u64,
    pub subject: String,
    pub action: String,
    pub target: String,
    pub allowed: bool,
}

pub struct AuditLog {
    path: PathBuf,
}

impl AuditLog {
    pub fn new(path: impl Into<PathBuf>) -> Self {
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

    pub fn query_by_subject(&self, subject: &str) -> std::io::Result<Vec<AuditEntry>> {
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|e| e.subject == subject)
            .collect())
    }

    pub fn query_by_target(&self, target_pattern: &str) -> std::io::Result<Vec<AuditEntry>> {
        let rule = TargetRule {
            pattern: target_pattern.to_string(),
            min_permission: Permission::ReadTarget,
        };
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|e| rule.matches(&e.target))
            .collect())
    }

    pub fn query_denials(&self) -> std::io::Result<Vec<AuditEntry>> {
        Ok(self
            .read_all()?
            .into_iter()
            .filter(|e| !e.allowed)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claims(roles: &[&str]) -> IdentityClaims {
        IdentityClaims {
            subject: "user_123".to_string(),
            email: "dev@company.com".to_string(),
            roles: roles.iter().map(|r| r.to_string()).collect(),
            issuer: "https://auth.company.com".to_string(),
            expires_at: 9999999999,
            audience: Some("fish-ci".to_string()),
            not_before: None,
        }
    }

    #[test]
    fn test_rbac_access_control() {
        let ac = AccessController::new();
        let user = claims(&["developer"]);
        assert!(ac.check_permission(&user, Permission::BuildTarget));
        assert!(!ac.check_permission(&user, Permission::AdminTarget));
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
    fn test_oidc_jwt_hmac_roundtrip() {
        let mut config = OidcConfig::default();
        config
            .allowed_issuers
            .push("https://auth.example.com".to_string());
        config.required_audience = Some("fish-worker".to_string());
        config
            .hmac_secrets
            .insert("key-1".to_string(), "super-secret-hmac-key".to_string());
        config
            .role_mappings
            .insert("admin-group".to_string(), "admin".to_string());

        let payload = serde_json::json!({
            "sub": "alice_456",
            "iss": "https://auth.example.com",
            "aud": "fish-worker",
            "email": "alice@example.com",
            "exp": 9999999999u64,
            "roles": ["admin-group"]
        });

        let token =
            create_signed_jwt_hmac(&payload, b"super-secret-hmac-key", Some("key-1")).unwrap();
        let validated = validate_oidc_jwt(&token, &config).unwrap();
        assert_eq!(validated.subject, "alice_456");
        assert_eq!(validated.roles, vec!["admin".to_string()]);
    }

    #[test]
    fn test_oidc_jwt_expired_rejection() {
        let mut config = OidcConfig::default();
        config
            .hmac_secrets
            .insert("key-1".to_string(), "secret".to_string());

        let payload = serde_json::json!({
            "sub": "bob",
            "iss": "test",
            "exp": 1000u64
        });

        let token = create_signed_jwt_hmac(&payload, b"secret", Some("key-1")).unwrap();
        let err = validate_oidc_jwt(&token, &config).unwrap_err();
        assert!(err.contains("expired"));
    }

    #[test]
    fn test_audit_log_querying() {
        let dir = tempfile::tempdir().unwrap();
        let log = AuditLog::new(dir.path().join("audit.jsonl"));

        log.record(&AuditEntry {
            timestamp_unix_secs: 100,
            subject: "user_a".to_string(),
            action: "BuildTarget".to_string(),
            target: "prod/api".to_string(),
            allowed: false,
        })
        .unwrap();

        log.record(&AuditEntry {
            timestamp_unix_secs: 101,
            subject: "user_b".to_string(),
            action: "BuildTarget".to_string(),
            target: "dev/api".to_string(),
            allowed: true,
        })
        .unwrap();

        let denials = log.query_denials().unwrap();
        assert_eq!(denials.len(), 1);
        assert_eq!(denials[0].subject, "user_a");

        let prod_entries = log.query_by_target("prod/*").unwrap();
        assert_eq!(prod_entries.len(), 1);
        assert_eq!(prod_entries[0].target, "prod/api");
    }
}
