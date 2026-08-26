//! Plugin Marketplace Registry — decentralized plugin discovery and
//! signed artifact distribution.
//!
//! The registry is a JSON index file hosted at any reachable URL. Each entry
//! carries the plugin's download URL, SHA-256 digest, and an Ed25519
//! signature over `name@version` + digest. Consumers verify signatures
//! against a configurable trust set before downloading and installing.

use std::path::{Path, PathBuf};

use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use sha2::Digest;

/// A single plugin entry in the registry index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    /// URL to download the `.wasm` binary.
    pub url: String,
    /// Hex-encoded SHA-256 of the downloaded artifact.
    pub sha256: String,
    /// Base64 Ed25519 signature over `"{name}@{version}:{sha256}"`.
    pub signature: String,
    /// Base64 Ed25519 public key of the signer.
    pub signer: String,
}

/// The full registry index document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistry {
    pub version: u32,
    #[serde(default)]
    pub plugins: Vec<RegistryEntry>,
}

impl PluginRegistry {
    /// Fetch and parse the registry index from a URL.
    pub fn fetch(endpoint: &str) -> Result<Self, String> {
        let offline = std::env::var("FISH_OFFLINE")
            .map(|v| {
                v == "1"
                    || v.eq_ignore_ascii_case("true")
                    || v.eq_ignore_ascii_case("yes")
                    || v.eq_ignore_ascii_case("on")
            })
            .unwrap_or(false);
        Self::fetch_with_offline(endpoint, offline)
    }

    pub fn fetch_with_offline(endpoint: &str, offline: bool) -> Result<Self, String> {
        if offline {
            return Err(
                "offline mode enabled (FISH_OFFLINE); plugin registry fetch rejected".to_string(),
            );
        }

        let resp = ureq::get(endpoint)
            .call()
            .map_err(|e| format!("registry fetch failed: {e}"))?;
        if resp.status() >= 400 {
            return Err(format!("registry returned HTTP {}", resp.status()));
        }
        let mut body_bytes = Vec::new();
        std::io::Read::read_to_end(&mut resp.into_reader(), &mut body_bytes)
            .map_err(|e| format!("registry read failed: {e}"))?;
        let body = String::from_utf8(body_bytes)
            .map_err(|e| format!("registry is not valid UTF-8: {e}"))?;
        serde_json::from_str(&body).map_err(|e| format!("invalid registry JSON: {e}"))
    }

    /// Search entries by name substring (case-insensitive).
    pub fn search(&self, query: &str) -> Vec<&RegistryEntry> {
        let lower = query.to_ascii_lowercase();
        self.plugins
            .iter()
            .filter(|e| e.name.to_ascii_lowercase().contains(&lower))
            .collect()
    }
}

/// Verify that an entry's signature matches its content fields.
pub fn verify_entry_signature(entry: &RegistryEntry) -> Result<(), String> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};

    let message = format!("{}@{}:{}", entry.name, entry.version, entry.sha256);
    let key_bytes: [u8; 32] = general_purpose::STANDARD
        .decode(&entry.signer)
        .map_err(|_| "invalid base64 in signer key".to_string())?
        .try_into()
        .map_err(|_| "signer key must be 32 bytes".to_string())?;
    let sig_bytes: [u8; 64] = general_purpose::STANDARD
        .decode(&entry.signature)
        .map_err(|_| "invalid base64 in signature".to_string())?
        .try_into()
        .map_err(|_| "signature must be 64 bytes".to_string())?;

    let verifying_key =
        VerifyingKey::from_bytes(&key_bytes).map_err(|e| format!("invalid signing key: {e}"))?;
    let sig =
        Signature::from_slice(&sig_bytes).map_err(|_| "invalid signature encoding".to_string())?;
    verifying_key
        .verify(message.as_bytes(), &sig)
        .map_err(|_| "signature verification failed".to_string())
}

/// Download a plugin artifact from a verified registry entry.
///
/// Verifies SHA-256 integrity after download. Returns raw WASM bytes.
pub fn download_plugin(entry: &RegistryEntry) -> Result<Vec<u8>, String> {
    let offline = std::env::var("FISH_OFFLINE")
        .map(|v| {
            v == "1"
                || v.eq_ignore_ascii_case("true")
                || v.eq_ignore_ascii_case("yes")
                || v.eq_ignore_ascii_case("on")
        })
        .unwrap_or(false);
    download_plugin_with_offline(entry, offline)
}

pub fn download_plugin_with_offline(
    entry: &RegistryEntry,
    offline: bool,
) -> Result<Vec<u8>, String> {
    if offline {
        return Err("offline mode enabled (FISH_OFFLINE); plugin download rejected".to_string());
    }

    let resp = ureq::get(&entry.url)
        .call()
        .map_err(|e| format!("plugin download failed: {e}"))?;
    if resp.status() >= 400 {
        return Err(format!("plugin download returned HTTP {}", resp.status()));
    }
    let mut bytes = Vec::new();
    std::io::Read::read_to_end(&mut resp.into_reader(), &mut bytes)
        .map_err(|e| format!("plugin read failed: {e}"))?;

    let actual: String = sha2::Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if actual != entry.sha256 {
        return Err(format!(
            "SHA-256 mismatch for `{}`: expected `{}`, got `{actual}`",
            entry.name, entry.sha256
        ));
    }
    Ok(bytes)
}

/// Install a downloaded plugin into the local `.fish/plugins/` directory.
pub fn install_plugin(
    name: &str,
    wasm_bytes: &[u8],
    plugins_dir: &Path,
) -> Result<PathBuf, String> {
    let plugin_dir = plugins_dir.join(name);
    std::fs::create_dir_all(&plugin_dir)
        .map_err(|e| format!("cannot create plugin dir {}: {e}", plugin_dir.display()))?;

    let dest = plugin_dir.join("plugin.wasm");
    std::fs::write(&dest, wasm_bytes).map_err(|e| format!("cannot write plugin binary: {e}"))?;

    // Generate a minimal manifest so discovery finds it.
    let manifest = serde_json::json!({
        "name": name,
        "version": "installed",
        "entrypoint": "plugin.wasm",
        "hooks": ["build"]
    });
    let manifest_path = plugin_dir.join("plugin.json");
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    )
    .map_err(|e| format!("cannot write plugin manifest: {e}"))?;

    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};
    use tempfile::tempdir;

    fn make_signed_entry(name: &str, version: &str, seed: &[u8; 32]) -> (RegistryEntry, String) {
        let signing_key = SigningKey::from_bytes(seed);
        let sha256: String = sha2::Sha256::digest(b"wasm-bytes")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let message = format!("{name}@{version}:{sha256}");
        let signature = signing_key.sign(message.as_bytes()).to_bytes();
        let public = signing_key.verifying_key().to_bytes();

        (
            RegistryEntry {
                name: name.to_string(),
                version: version.to_string(),
                description: Some("test plugin".to_string()),
                url: "https://example.com/plugin.wasm".to_string(),
                sha256,
                signature: general_purpose::STANDARD.encode(signature),
                signer: general_purpose::STANDARD.encode(public),
            },
            b64(&public),
        )
    }

    fn b64(bytes: &[u8]) -> String {
        general_purpose::STANDARD.encode(bytes)
    }

    fn make_stub(name: &str) -> RegistryEntry {
        RegistryEntry {
            name: name.to_string(),
            version: "1.0.0".to_string(),
            description: None,
            url: String::new(),
            sha256: String::new(),
            signature: String::new(),
            signer: String::new(),
        }
    }

    #[test]
    fn test_verify_entry_signature_accepts_valid() {
        let (entry, _) = make_signed_entry("proto-gen", "1.0.0", &[1u8; 32]);
        assert!(verify_entry_signature(&entry).is_ok());
    }

    #[test]
    fn test_verify_entry_signature_rejects_tampered() {
        let (mut entry, _) = make_signed_entry("proto-gen", "1.0.0", &[1u8; 32]);
        entry.sha256 = sha2::Sha256::digest(b"tampered")
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert!(verify_entry_signature(&entry).is_err());
    }

    #[test]
    fn test_search_by_name_substring() {
        let registry = PluginRegistry {
            version: 1,
            plugins: vec![make_stub("proto-gen"), make_stub("linter")],
        };

        assert_eq!(registry.search("proto").len(), 1);
        assert_eq!(registry.search("LINT").len(), 1);
        assert_eq!(registry.search("nonexistent").len(), 0);
    }

    #[test]
    fn test_install_plugin_creates_directory_and_manifest() {
        let dir = tempdir().unwrap();
        let plugins_dir = dir.path().join(".fish/plugins");

        let dest = install_plugin("my_plugin", b"\0asm\x01\0\0\0", &plugins_dir).unwrap();

        assert!(dest.exists());
        assert!(dest.parent().unwrap().join("plugin.json").exists());

        let manifest: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dest.parent().unwrap().join("plugin.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["name"], "my_plugin");
    }

    #[test]
    fn test_offline_plugin_fetch_fail_fast() {
        let err = PluginRegistry::fetch_with_offline("http://example.com/registry.json", true)
            .unwrap_err();
        assert!(err.contains("offline mode"));

        let stub = make_stub("demo");
        let dl_err = download_plugin_with_offline(&stub, true).unwrap_err();
        assert!(dl_err.contains("offline mode"));
    }
}
