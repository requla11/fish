use std::path::{Path, PathBuf};

use base64::{Engine as _, engine::general_purpose};
use serde::{Deserialize, Serialize};
use sha2::Digest;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub url: String,
    pub sha256: String,
    pub signature: String,
    pub signer: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InstalledPluginInfo {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub manifest_path: PathBuf,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginRegistry {
    pub version: u32,
    #[serde(default)]
    pub plugins: Vec<RegistryEntry>,
}

impl PluginRegistry {
    pub fn new(version: u32, plugins: Vec<RegistryEntry>) -> Self {
        Self { version, plugins }
    }

    pub fn fetch(endpoint: &str) -> Result<Self, String> {
        let offline = std::env::var("FISH_OFFLINE").is_ok_and(|v| {
            v == "1"
                || v.eq_ignore_ascii_case("true")
                || v.eq_ignore_ascii_case("yes")
                || v.eq_ignore_ascii_case("on")
        });
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

    pub fn search(&self, query: &str) -> Vec<&RegistryEntry> {
        let lower = query.to_ascii_lowercase();
        self.plugins
            .iter()
            .filter(|e| {
                e.name.to_ascii_lowercase().contains(&lower)
                    || e.description
                        .as_ref()
                        .is_some_and(|d| d.to_ascii_lowercase().contains(&lower))
            })
            .collect()
    }

    pub fn find(&self, name: &str, version: Option<&str>) -> Option<&RegistryEntry> {
        self.plugins.iter().find(|e| {
            if !e.name.eq_ignore_ascii_case(name) {
                return false;
            }
            if let Some(ver) = version {
                e.version == ver
            } else {
                true
            }
        })
    }

    pub fn save_to_cache(&self, cache_path: &Path) -> Result<(), String> {
        if let Some(parent) = cache_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create cache dir {}: {e}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("registry serialization failed: {e}"))?;
        std::fs::write(cache_path, json)
            .map_err(|e| format!("cannot write cache file {}: {e}", cache_path.display()))?;
        Ok(())
    }

    pub fn load_from_cache(cache_path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(cache_path)
            .map_err(|e| format!("cannot read cache file {}: {e}", cache_path.display()))?;
        serde_json::from_str(&content).map_err(|e| format!("invalid cached registry JSON: {e}"))
    }
}

pub fn create_signed_entry(
    name: &str,
    version: &str,
    description: Option<String>,
    url: &str,
    wasm_bytes: &[u8],
    signing_seed: &[u8; 32],
) -> Result<RegistryEntry, String> {
    use ed25519_dalek::{Signer, SigningKey};

    let sha256: String = sha2::Sha256::digest(wasm_bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    let signing_key = SigningKey::from_bytes(signing_seed);
    let message = format!("{name}@{version}:{sha256}");
    let signature_bytes = signing_key.sign(message.as_bytes()).to_bytes();
    let public_bytes = signing_key.verifying_key().to_bytes();

    Ok(RegistryEntry {
        name: name.to_string(),
        version: version.to_string(),
        description,
        url: url.to_string(),
        sha256,
        signature: general_purpose::STANDARD.encode(signature_bytes),
        signer: general_purpose::STANDARD.encode(public_bytes),
    })
}

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

pub fn verify_entry_with_trusted_keys(
    entry: &RegistryEntry,
    trusted_keys: &[String],
) -> Result<(), String> {
    verify_entry_signature(entry)?;

    if trusted_keys.is_empty() {
        return Ok(());
    }

    let signer_trimmed = entry.signer.trim();
    let is_trusted = trusted_keys.iter().any(|k| {
        let k_trimmed = k.trim();
        if k_trimmed.eq_ignore_ascii_case(signer_trimmed) {
            return true;
        }

        if let (Ok(k_raw), Ok(signer_raw)) = (
            general_purpose::STANDARD.decode(k_trimmed),
            general_purpose::STANDARD.decode(signer_trimmed),
        ) {
            return k_raw == signer_raw;
        }

        false
    });

    if !is_trusted {
        return Err(format!(
            "plugin `{}` signed by untrusted key `{}`",
            entry.name, entry.signer
        ));
    }

    Ok(())
}

pub fn download_plugin(entry: &RegistryEntry) -> Result<Vec<u8>, String> {
    let offline = std::env::var("FISH_OFFLINE").is_ok_and(|v| {
        v == "1"
            || v.eq_ignore_ascii_case("true")
            || v.eq_ignore_ascii_case("yes")
            || v.eq_ignore_ascii_case("on")
    });
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

pub fn uninstall_plugin(name: &str, plugins_dir: &Path) -> Result<bool, String> {
    let plugin_dir = plugins_dir.join(name);
    if !plugin_dir.exists() {
        return Ok(false);
    }
    std::fs::remove_dir_all(&plugin_dir)
        .map_err(|e| format!("cannot remove plugin dir {}: {e}", plugin_dir.display()))?;
    Ok(true)
}

pub fn list_installed_plugins(plugins_dir: &Path) -> Result<Vec<InstalledPluginInfo>, String> {
    if !plugins_dir.exists() {
        return Ok(Vec::new());
    }

    let mut result = Vec::new();
    let entries =
        std::fs::read_dir(plugins_dir).map_err(|e| format!("cannot read plugins dir: {e}"))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let plugin_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default()
                .to_string();
            let wasm_file = path.join("plugin.wasm");
            let manifest_path = path.join("plugin.json");

            if wasm_file.exists() {
                let size_bytes = wasm_file.metadata().map(|m| m.len()).unwrap_or(0);
                let version = if manifest_path.exists() {
                    std::fs::read_to_string(&manifest_path)
                        .ok()
                        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
                        .and_then(|v| {
                            v.get("version")
                                .and_then(|ver| ver.as_str())
                                .map(ToString::to_string)
                        })
                        .unwrap_or_else(|| "installed".to_string())
                } else {
                    "installed".to_string()
                };

                result.push(InstalledPluginInfo {
                    name: plugin_name,
                    version,
                    path: wasm_file,
                    manifest_path,
                    size_bytes,
                });
            }
        }
    }

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn make_signed_entry(name: &str, version: &str, seed: &[u8; 32]) -> (RegistryEntry, String) {
        let entry = create_signed_entry(
            name,
            version,
            Some("test plugin".to_string()),
            "https://example.com/plugin.wasm",
            b"wasm-bytes",
            seed,
        )
        .unwrap();
        let signer = entry.signer.clone();
        (entry, signer)
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
    fn test_verify_entry_with_trusted_keys() {
        let (entry, signer) = make_signed_entry("proto-gen", "1.0.0", &[2u8; 32]);
        assert!(verify_entry_with_trusted_keys(&entry, &[signer.clone()]).is_ok());
        assert!(verify_entry_with_trusted_keys(&entry, &[]).is_ok());
        let untrusted = "dGVzdC11bnRydXN0ZWQta2V5LWZvci10ZXN0aW5nMTIzNDU=".to_string();
        assert!(verify_entry_with_trusted_keys(&entry, &[untrusted]).is_err());
    }

    #[test]
    fn test_search_and_find_plugins() {
        let registry = PluginRegistry::new(
            1,
            vec![
                RegistryEntry {
                    name: "proto-gen".to_string(),
                    version: "1.0.0".to_string(),
                    description: Some("Protobuf codegen compiler plugin".to_string()),
                    url: "https://example.com/proto.wasm".to_string(),
                    sha256: "abc".to_string(),
                    signature: "sig".to_string(),
                    signer: "signer".to_string(),
                },
                make_stub("linter"),
            ],
        );

        assert_eq!(registry.search("proto").len(), 1);
        assert_eq!(registry.search("compiler").len(), 1);
        assert_eq!(registry.search("LINT").len(), 1);
        assert_eq!(registry.search("nonexistent").len(), 0);

        assert!(registry.find("proto-gen", Some("1.0.0")).is_some());
        assert!(registry.find("proto-gen", Some("2.0.0")).is_none());
        assert!(registry.find("proto-gen", None).is_some());
        assert!(registry.find("missing", None).is_none());
    }

    #[test]
    fn test_install_list_and_uninstall_plugin() {
        let dir = tempdir().unwrap();
        let plugins_dir = dir.path().join(".fish/plugins");

        let dest = install_plugin("my_plugin", b"\0asm\x01\0\0\0", &plugins_dir).unwrap();
        assert!(dest.exists());
        assert!(dest.parent().unwrap().join("plugin.json").exists());

        let list = list_installed_plugins(&plugins_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "my_plugin");
        assert_eq!(list[0].size_bytes, 8);

        let uninstalled = uninstall_plugin("my_plugin", &plugins_dir).unwrap();
        assert!(uninstalled);
        assert!(!dest.exists());

        let list_after = list_installed_plugins(&plugins_dir).unwrap();
        assert!(list_after.is_empty());

        let uninstalled_again = uninstall_plugin("my_plugin", &plugins_dir).unwrap();
        assert!(!uninstalled_again);
    }

    #[test]
    fn test_save_and_load_cache() {
        let dir = tempdir().unwrap();
        let cache_file = dir.path().join("cache/registry.json");

        let registry = PluginRegistry::new(1, vec![make_stub("cached-plugin")]);
        registry.save_to_cache(&cache_file).unwrap();
        assert!(cache_file.exists());

        let loaded = PluginRegistry::load_from_cache(&cache_file).unwrap();
        assert_eq!(loaded.version, 1);
        assert_eq!(loaded.plugins.len(), 1);
        assert_eq!(loaded.plugins[0].name, "cached-plugin");
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
