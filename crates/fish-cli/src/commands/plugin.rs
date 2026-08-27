use std::path::Path;
use std::process::ExitCode;

use fish_plugin::marketplace::{
    PluginRegistry, create_signed_entry, download_plugin, install_plugin, list_installed_plugins,
    uninstall_plugin, verify_entry_signature, verify_entry_with_trusted_keys,
};
use fish_plugin::scripting::PluginError;

use crate::args::{PluginAction, PluginArgs};
use crate::backends;

const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/requla11/fish-registry/main/index.json";

fn resolve_registry(endpoint: Option<&str>, cache_path: &Path) -> Result<PluginRegistry, String> {
    let env_url = std::env::var("FISH_PLUGIN_REGISTRY").ok();
    let url = endpoint
        .or(env_url.as_deref())
        .unwrap_or(DEFAULT_REGISTRY_URL);
    match PluginRegistry::fetch(url) {
        Ok(reg) => {
            let _ = reg.save_to_cache(cache_path);
            Ok(reg)
        }
        Err(err) => {
            if cache_path.exists() {
                PluginRegistry::load_from_cache(cache_path)
            } else {
                Err(err)
            }
        }
    }
}


pub fn run_plugin(args: PluginArgs) -> ExitCode {
    let start_dir = match crate::utils::resolve_start_dir(args.path.as_deref()) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let plugins_dir = start_dir.join(".fish").join("plugins");
    let cache_file = start_dir.join(".fish").join("cache").join("registry.json");

    match args.action {
        PluginAction::List => {
            let wasm_plugins = list_installed_plugins(&plugins_dir).unwrap_or_default();
            let script_plugins = backends::list_script_plugins(&start_dir);

            if wasm_plugins.is_empty() && script_plugins.is_empty() {
                println!("No plugins found in .fish/plugins/");
                return ExitCode::SUCCESS;
            }

            if !wasm_plugins.is_empty() {
                println!("Installed WASM / Native Plugins:");
                for p in &wasm_plugins {
                    println!(
                        "  - {} (v{}) [{} bytes] -> {}",
                        p.name,
                        p.version,
                        p.size_bytes,
                        p.path.display()
                    );
                }
            }

            if !script_plugins.is_empty() {
                println!("Available Script Plugins:");
                for plugin in &script_plugins {
                    println!("  - {}", plugin);
                }
            }

            ExitCode::SUCCESS
        }
        PluginAction::Search { query, registry } => {
            let registry_doc = match resolve_registry(registry.as_deref(), &cache_file) {
                Ok(doc) => doc,
                Err(err) => {
                    eprintln!("error: cannot load plugin registry: {err}");
                    return ExitCode::FAILURE;
                }
            };

            let results = match query {
                Some(ref q) if !q.trim().is_empty() => registry_doc.search(q.trim()),
                _ => registry_doc.plugins.iter().collect(),
            };

            if results.is_empty() {
                println!("No matching plugins found in registry.");
            } else {
                println!("Found {} plugin(s):", results.len());
                for entry in results {
                    let desc = entry.description.as_deref().unwrap_or("No description");
                    println!(
                        "  - {} (v{}) : {}\n      url: {}\n      signer: {}",
                        entry.name, entry.version, desc, entry.url, entry.signer
                    );
                }
            }

            ExitCode::SUCCESS
        }
        PluginAction::Install {
            name,
            version,
            registry,
            trusted_key,
            insecure,
        } => {
            let registry_doc = match resolve_registry(registry.as_deref(), &cache_file) {
                Ok(doc) => doc,
                Err(err) => {
                    eprintln!("error: cannot load plugin registry: {err}");
                    return ExitCode::FAILURE;
                }
            };

            let entry = match registry_doc.find(&name, version.as_deref()) {
                Some(e) => e,
                None => {
                    eprintln!(
                        "error: plugin '{}'{} not found in registry",
                        name,
                        version
                            .map(|v| format!(" (version {v})"))
                            .unwrap_or_default()
                    );
                    return ExitCode::FAILURE;
                }
            };

            println!(
                "Verifying Ed25519 signature for plugin '{}' (v{})...",
                entry.name, entry.version
            );
            if let Err(err) = verify_entry_signature(entry) {
                eprintln!("error: invalid plugin signature: {err}");
                return ExitCode::FAILURE;
            }

            if !insecure {
                let mut trusted_keys = Vec::new();
                if let Some(ref k) = trusted_key {
                    trusted_keys.push(k.clone());
                }
                if let Ok(env_keys) = std::env::var("FISH_TRUSTED_KEYS") {
                    for k in env_keys.split(',') {
                        let trimmed = k.trim();
                        if !trimmed.is_empty() {
                            trusted_keys.push(trimmed.to_string());
                        }
                    }
                }

                if !trusted_keys.is_empty() {
                    if let Err(err) = verify_entry_with_trusted_keys(entry, &trusted_keys) {
                        eprintln!("error: trust verification failed: {err}");
                        return ExitCode::FAILURE;
                    }
                }
            }

            println!("Downloading plugin artifact from {}...", entry.url);
            let wasm_bytes = match download_plugin(entry) {
                Ok(bytes) => bytes,
                Err(err) => {
                    eprintln!("error: download failed: {err}");
                    return ExitCode::FAILURE;
                }
            };

            match install_plugin(&entry.name, &wasm_bytes, &plugins_dir) {
                Ok(dest) => {
                    println!(
                        "Successfully installed plugin '{}' (v{}) to {}",
                        entry.name,
                        entry.version,
                        dest.display()
                    );
                    ExitCode::SUCCESS
                }
                Err(err) => {
                    eprintln!("error: installation failed: {err}");
                    return ExitCode::FAILURE;
                }
            }
        }
        PluginAction::Uninstall { name } => match uninstall_plugin(&name, &plugins_dir) {
            Ok(true) => {
                println!("Successfully uninstalled plugin '{}'", name);
                ExitCode::SUCCESS
            }
            Ok(false) => {
                eprintln!("error: plugin '{}' is not installed", name);
                ExitCode::FAILURE
            }
            Err(err) => {
                eprintln!("error: uninstall failed: {err}");
                return ExitCode::FAILURE;
            }
        },
        PluginAction::Publish {
            wasm_path,
            name,
            version,
            url,
            description,
            seed,
        } => {
            let wasm_bytes = match std::fs::read(&wasm_path) {
                Ok(bytes) => bytes,
                Err(err) => {
                    eprintln!(
                        "error: cannot read WASM file {}: {err}",
                        wasm_path.display()
                    );
                    return ExitCode::FAILURE;
                }
            };

            let seed_str = match seed.or_else(|| std::env::var("FISH_SIGNING_SEED").ok()) {
                Some(s) if !s.trim().is_empty() => s,
                _ => {
                    eprintln!("error: signing seed required (--seed or FISH_SIGNING_SEED)");
                    return ExitCode::FAILURE;
                }
            };

            let mut seed_bytes = [0u8; 32];
            if hex::decode_to_slice(seed_str.trim(), &mut seed_bytes).is_err() {
                eprintln!("error: seed must be a 64-character hex string (32 bytes)");
                return ExitCode::FAILURE;
            }

            match create_signed_entry(&name, &version, description, &url, &wasm_bytes, &seed_bytes)
            {
                Ok(entry) => match serde_json::to_string_pretty(&entry) {
                    Ok(json) => {
                        println!("{json}");
                        ExitCode::SUCCESS
                    }
                    Err(err) => {
                        eprintln!("error: JSON serialization failed: {err}");
                        ExitCode::FAILURE
                    }
                },
                Err(err) => {
                    eprintln!("error: entry signing failed: {err}");
                    ExitCode::FAILURE
                }
            }
        }
        PluginAction::Execute {
            name,
            command,
            args: plugin_args,
        } => match backends::execute_script_plugin(&start_dir, &name, &command, &plugin_args) {
            Ok(output) => {
                if !output.stdout.is_empty() {
                    print!("{}", output.stdout);
                }
                if !output.stderr.is_empty() {
                    eprint!("{}", output.stderr);
                }
                if output.success {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(PluginError::InvalidConfig(msg)) => {
                eprintln!("error: {}", msg);
                ExitCode::FAILURE
            }
            Err(PluginError::Execution {
                command: cmd,
                message,
            }) => {
                eprintln!("error: plugin '{}' failed: {}", cmd, message);
                ExitCode::FAILURE
            }
            Err(PluginError::Unsupported(msg)) => {
                eprintln!("error: {}", msg);
                ExitCode::FAILURE
            }
            Err(PluginError::DependencyMissing(dep)) => {
                eprintln!("error: missing plugin dependency: {}", dep);
                ExitCode::FAILURE
            }
        },
    }
}
