use crate::wasm::WasmPluginManifest;

/// Risk level assigned to a plugin manifest after capability analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// One concrete reason a manifest was flagged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: RiskLevel,
    pub field: &'static str,
    pub message: String,
}

impl Finding {
    fn new(severity: RiskLevel, field: &'static str, message: impl Into<String>) -> Self {
        Self {
            severity,
            field,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityAudit {
    pub plugin: String,
    pub risk: RiskLevel,
    pub findings: Vec<Finding>,
}

impl CapabilityAudit {
    /// Audit passes when nothing above Medium risk was flagged.
    pub fn acceptable(&self) -> bool {
        self.risk <= RiskLevel::Medium
    }
}

const SECRET_TOKENS: [&str; 9] = [
    "TOKEN",
    "SECRET",
    "PASSWORD",
    "PASSWD",
    "PRIVATE_KEY",
    "AWS_ACCESS_KEY_ID",
    "AWS_SECRET_ACCESS_KEY",
    "GITHUB_TOKEN",
    "API_KEY",
];

fn is_dangerous_read_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    matches!(normalized.as_str(), "/" | "." | ".." | "~")
        || normalized == "$HOME"
        || normalized.contains("**")
        || normalized.starts_with("/etc")
        || normalized.starts_with("/root")
        || normalized.to_ascii_lowercase().contains("ssh")
}

fn is_dangerous_write_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    matches!(
        normalized.as_str(),
        "/" | "." | ".." | "~" | "/etc" | "/usr" | "/bin"
    ) || normalized.contains("**")
        || normalized.ends_with("/src")
        || normalized == "src"
        || normalized.contains("Cargo.toml")
        || normalized.contains(".git")
}

/// Static capability analysis of a plugin manifest.
///
/// Flags overly broad filesystem access, secret-bearing environment grants,
/// and oversized resource limits *before* a plugin is ever executed. The
/// auditor inspects declarations only; it cannot prove runtime behavior.
pub fn audit_manifest(manifest: &WasmPluginManifest) -> CapabilityAudit {
    let mut findings = Vec::new();

    for read in &manifest.capabilities.allow_read_paths {
        if is_dangerous_read_path(read) {
            findings.push(Finding::new(
                RiskLevel::High,
                "capabilities.allow_read_paths",
                format!("read access `{read}` reaches far beyond project inputs"),
            ));
        }
    }

    for write in &manifest.capabilities.allow_write_paths {
        if is_dangerous_write_path(write) {
            findings.push(Finding::new(
                RiskLevel::Critical,
                "capabilities.allow_write_paths",
                format!("write access `{write}` can mutate build inputs or system paths"),
            ));
        } else if PathLike(write).is_absolute() {
            findings.push(Finding::new(
                RiskLevel::Medium,
                "capabilities.allow_write_paths",
                format!("absolute write target `{write}` escapes the workspace"),
            ));
        }
    }

    for var in &manifest.capabilities.allow_env_vars {
        let upper = var.to_ascii_uppercase();
        if SECRET_TOKENS.iter().any(|token| upper.contains(token)) {
            findings.push(Finding::new(
                RiskLevel::High,
                "capabilities.allow_env_vars",
                format!("environment grant `{var}` likely exposes a credential"),
            ));
        }
    }

    // 64 KiB per page: 1024 pages = 64 MiB, generous for build plugins.
    const MAX_SANE_PAGES: u32 = 4096;
    if manifest.capabilities.max_memory_pages > MAX_SANE_PAGES {
        findings.push(Finding::new(
            RiskLevel::Medium,
            "capabilities.max_memory_pages",
            format!(
                "{} pages (~{} MiB) far exceeds typical plugin needs",
                manifest.capabilities.max_memory_pages,
                (manifest.capabilities.max_memory_pages as u64 * 64) / 1024
            ),
        ));
    }

    if manifest.capabilities.max_execution_time_ms > 300_000 {
        findings.push(Finding::new(
            RiskLevel::Medium,
            "capabilities.max_execution_time_ms",
            format!(
                "{} ms execution budget exceeds the 5 minute guideline",
                manifest.capabilities.max_execution_time_ms
            ),
        ));
    }

    CapabilityAudit {
        plugin: manifest.name.clone(),
        risk: findings
            .iter()
            .map(|f| f.severity)
            .max()
            .unwrap_or(RiskLevel::Low),
        findings,
    }
}

struct PathLike<'a>(&'a str);

impl PathLike<'_> {
    /// Host-OS independent absoluteness check: audit rules must behave the
    /// same on Windows runners as on Linux, so a POSIX-style leading slash
    /// counts even where `std::path` would disagree.
    fn is_absolute(&self) -> bool {
        let s = self.0.as_bytes();
        if s.is_empty() {
            return false;
        }
        if s[0] == b'/' || s[0] == b'\\' {
            return true;
        }
        s.len() >= 2 && s[1] == b':' && s[0].is_ascii_alphabetic()
    }
}

/// Audit every plugin in a registry and return them sorted worst-first.
pub fn audit_registry(registry: &crate::wasm::WasmPluginRegistry) -> Vec<CapabilityAudit> {
    let mut audits: Vec<CapabilityAudit> = registry
        .plugin_names()
        .into_iter()
        .filter_map(|name| registry.get(&name))
        .map(|engine| audit_manifest(engine.manifest()))
        .collect();
    audits.sort_by(|a, b| b.risk.cmp(&a.risk).then(a.plugin.cmp(&b.plugin)));
    audits
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::WasmCapabilities;

    fn manifest(capabilities: WasmCapabilities) -> WasmPluginManifest {
        WasmPluginManifest {
            name: "test-plugin".to_string(),
            version: "1.0.0".to_string(),
            entrypoint: "plugin.wasm".to_string(),
            description: None,
            hooks: vec!["build".to_string()],
            capabilities,
        }
    }

    #[test]
    fn test_clean_manifest_passes() {
        let caps = WasmCapabilities {
            allow_read_paths: vec!["proto".to_string(), "src/api".to_string()],
            allow_write_paths: vec!["target/wasm_out".to_string()],
            allow_env_vars: vec!["PROTOC_PATH".to_string(), "RUST_LOG".to_string()],
            max_memory_pages: 256,
            max_execution_time_ms: 10_000,
        };
        let audit = audit_manifest(&manifest(caps));
        assert_eq!(audit.risk, RiskLevel::Low);
        assert!(audit.findings.is_empty());
        assert!(audit.acceptable());
    }

    #[test]
    fn test_wildcard_write_is_critical() {
        let caps = WasmCapabilities {
            allow_write_paths: vec!["**".to_string()],
            ..WasmCapabilities::default()
        };
        let audit = audit_manifest(&manifest(caps));
        assert_eq!(audit.risk, RiskLevel::Critical);
        assert!(!audit.acceptable());
        assert_eq!(audit.findings[0].field, "capabilities.allow_write_paths");
    }

    #[test]
    fn test_secret_env_vars_are_flagged() {
        for candidate in ["AWS_SECRET_ACCESS_KEY", "MY_API_KEY", "GITHUB_TOKEN"] {
            let caps = WasmCapabilities {
                allow_env_vars: vec![candidate.to_string()],
                ..WasmCapabilities::default()
            };
            let audit = audit_manifest(&manifest(caps));
            assert_eq!(audit.risk, RiskLevel::High, "{candidate}");
            assert!(!audit.acceptable());
        }
    }

    #[test]
    fn test_root_read_and_absolute_write() {
        let caps = WasmCapabilities {
            allow_read_paths: vec!["/".to_string(), "/home/user/.ssh".to_string()],
            allow_write_paths: vec!["/tmp/out".to_string()],
            ..WasmCapabilities::default()
        };
        let audit = audit_manifest(&manifest(caps));
        assert_eq!(audit.risk, RiskLevel::High);
        assert_eq!(audit.findings.len(), 3);
        let fields: Vec<_> = audit.findings.iter().map(|f| f.field).collect();
        assert!(fields.contains(&"capabilities.allow_write_paths"));
    }

    #[test]
    fn test_oversized_limits_are_medium_risk() {
        let caps = WasmCapabilities {
            max_memory_pages: 16_384,
            max_execution_time_ms: 900_000,
            ..WasmCapabilities::default()
        };
        let audit = audit_manifest(&manifest(caps));
        assert_eq!(audit.risk, RiskLevel::Medium);
        assert!(
            audit.acceptable(),
            "resource over-provisioning alone still ships"
        );
    }

    #[test]
    fn test_git_and_source_writes_rejected() {
        let caps = WasmCapabilities {
            allow_write_paths: vec![".git/hooks".to_string(), "src".to_string()],
            ..WasmCapabilities::default()
        };
        let audit = audit_manifest(&manifest(caps));
        assert_eq!(audit.risk, RiskLevel::Critical);
        assert_eq!(audit.findings.len(), 2);
    }
}
