//! Declarative sandbox security profiles.
//!
//! Named presets (`strict`, `default`, `trusted`) that map to concrete
//! [`SecurityPolicy`] configurations. Profiles give users a single knob
//! instead of requiring them to understand every individual policy field.

use crate::security::{SecurityLevel, SecurityPolicy};

/// Well-known sandbox profile names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SandboxProfile {
    /// Allow everything — for trusted internal tooling only.
    Trusted,
    /// Default: block suspicious arguments and path traversal.
    Standard,
    /// Fail-closed: explicit allow-list required for every operation.
    Strict,
}

impl SandboxProfile {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().as_str() {
            "trusted" => Some(Self::Trusted),
            "default" | "standard" => Some(Self::Standard),
            "strict" => Some(Self::Strict),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Trusted => "trusted",
            Self::Standard => "default",
            Self::Strict => "strict",
        }
    }

    /// Build the [`SecurityPolicy`] for this profile.
    ///
    /// `allowed_paths` seeds the allow-list used by `Strict`; pass
    /// workspace-relative source/output dirs. `Standard` also registers them
    /// but still permits non-listed paths.
    pub fn build_policy(self, allowed_paths: &[String]) -> SecurityPolicy {
        let level = match self {
            Self::Trusted => SecurityLevel::AllowAll,
            Self::Standard => SecurityLevel::Paranoid,
            Self::Strict => SecurityLevel::Strict,
        };
        let mut policy = SecurityPolicy::new(level);
        for path in allowed_paths {
            policy.add_allowed_path(std::path::PathBuf::from(path));
        }
        policy
    }
}

/// Parse a profile name from a config string.
pub fn parse_profile(name: &str) -> Result<SandboxProfile, String> {
    SandboxProfile::from_name(name).ok_or_else(|| {
        format!("unknown sandbox profile `{name}`; expected one of: strict, default, trusted")
    })
}
