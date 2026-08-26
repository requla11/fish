use std::path::Path;

use fish_core::{DEFAULT_EXCLUDED_DIRS, FingerprintUtils};

use crate::config::GoProjectConfig;

pub fn compute_go_fingerprint(
    project_dir: &Path,
    toolchain_version: &str,
    config: &GoProjectConfig,
) -> Result<String, std::io::Error> {
    let mut hasher = blake3::Hasher::new();
    hasher.update(toolchain_version.as_bytes());
    hasher.update(b"\ntags:\n");
    let mut tags: Vec<&String> = config.tags.iter().collect();
    tags.sort();
    for tag in tags {
        hasher.update(tag.as_bytes());
        hasher.update(b"\n");
    }

    // Build-shaping flags and injected env change the produced artifacts;
    // leaving them out would let one configuration serve another's cached
    // build (e.g. flipping -race).
    hasher.update(b"\nbuild-config:\n");
    hasher.update(format!("race={}\n", config.race).as_bytes());
    hasher.update(format!("coverage={}\n", config.coverage).as_bytes());
    hasher.update(format!("ldflags={:?}\n", config.ldflags.as_deref().unwrap_or("")).as_bytes());
    hasher.update(format!("gcflags={:?}\n", config.gcflags.as_deref().unwrap_or("")).as_bytes());
    let mut env_pairs: Vec<(&String, &String)> = config.env.iter().collect();
    env_pairs.sort();
    for (key, value) in env_pairs {
        hasher.update(key.as_bytes());
        hasher.update(b"=");
        hasher.update(value.as_bytes());
        hasher.update(b"\n");
    }

    let go_mod = project_dir.join("go.mod");
    if go_mod.exists() {
        let _ = FingerprintUtils::hash_file_into(&go_mod, &mut hasher);
    }

    let go_sum = project_dir.join("go.sum");
    if go_sum.exists() {
        let _ = FingerprintUtils::hash_file_into(&go_sum, &mut hasher);
    }

    let go_work = project_dir.join("go.work");
    if go_work.exists() {
        let _ = FingerprintUtils::hash_file_into(&go_work, &mut hasher);
    }

    FingerprintUtils::hash_directory_with_extensions(
        project_dir,
        &["go"],
        DEFAULT_EXCLUDED_DIRS,
        &mut hasher,
    )?;

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GoProjectConfig;
    use std::collections::HashMap;
    use std::fs;
    use tempfile::tempdir;

    fn sample_config(race: bool) -> GoProjectConfig {
        GoProjectConfig {
            name: "demo".to_string(),
            package_path: "./...".to_string(),
            tags: vec![],
            ldflags: None,
            gcflags: None,
            run_tests: true,
            race,
            coverage: false,
            run_benchmarks: false,
            run_linter: false,
            output_binary: None,
            env: HashMap::new(),
        }
    }

    #[test]
    fn test_go_fingerprint_distinguishes_race_mode() {
        let temp = tempdir().unwrap();
        fs::write(
            temp.path().join("main.go"),
            "package main\nfunc main() {}\n",
        )
        .unwrap();

        let debug = compute_go_fingerprint(temp.path(), "1.22", &sample_config(false)).unwrap();
        let raced = compute_go_fingerprint(temp.path(), "1.22", &sample_config(true)).unwrap();

        assert_ne!(debug, raced);
    }
}
