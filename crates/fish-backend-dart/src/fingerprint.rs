use std::path::Path;

use crate::DartBackendError;
use crate::config::DartProjectType;
use fish_core::{DEFAULT_EXCLUDED_DIRS, FingerprintUtils};

pub fn compute_dart_fingerprint(
    project_dir: &Path,
    dart_version: &str,
    flutter_version: &Option<String>,
    project_type: &DartProjectType,
    target_platform: &str,
    release: bool,
) -> Result<String, DartBackendError> {
    let mut hasher = blake3::Hasher::new();

    hasher.update(dart_version.as_bytes());
    if let Some(flutter_ver) = flutter_version {
        hasher.update(flutter_ver.as_bytes());
    }
    // Target platform and optimize mode change the emitted build commands;
    // ignoring them would replay another target's cached artifacts.
    hasher.update(target_platform.as_bytes());
    hasher.update(if release { b"release" } else { b"debug" });

    match project_type {
        DartProjectType::Dart => {
            hasher.update(b"dart");
        }
        DartProjectType::Flutter => {
            hasher.update(b"flutter");
        }
    }

    let extensions = &["dart", "yaml", "json", "arb"];
    FingerprintUtils::hash_directory_with_extensions(
        project_dir,
        extensions,
        DEFAULT_EXCLUDED_DIRS,
        &mut hasher,
    )
    .map_err(DartBackendError::Io)?;

    for name in &[
        "pubspec.yaml",
        "pubspec.lock",
        "analysis_options.yaml",
        ".metadata",
        ".packages",
    ] {
        let path = project_dir.join(name);
        if path.exists() {
            let _ = FingerprintUtils::hash_file_into(&path, &mut hasher);
        }
    }

    let pkg_config = project_dir.join(".dart_tool").join("package_config.json");
    if pkg_config.exists() {
        let _ = FingerprintUtils::hash_file_into(&pkg_config, &mut hasher);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_dart_fingerprint() {
        let temp = tempdir().unwrap();
        let lib_dir = temp.path().join("lib");
        fs::create_dir_all(&lib_dir).unwrap();

        let dart_file = lib_dir.join("main.dart");
        fs::write(&dart_file, "void main() { print('Hello'); }").unwrap();

        let pubspec_file = temp.path().join("pubspec.yaml");
        fs::write(&pubspec_file, "name: test_app").unwrap();

        let fingerprint = compute_dart_fingerprint(
            temp.path(),
            "3.0.0",
            &None,
            &DartProjectType::Dart,
            "native",
            false,
        )
        .unwrap();

        assert!(!fingerprint.is_empty());
        assert_eq!(fingerprint.len(), 64);
    }

    #[test]
    fn test_fingerprint_changes_with_source() {
        let temp = tempdir().unwrap();
        let lib_dir = temp.path().join("lib");
        fs::create_dir_all(&lib_dir).unwrap();

        let dart_file = lib_dir.join("main.dart");
        fs::write(&dart_file, "void main() { print('Hello'); }").unwrap();

        let fp1 = compute_dart_fingerprint(
            temp.path(),
            "3.0.0",
            &None,
            &DartProjectType::Dart,
            "native",
            false,
        )
        .unwrap();

        fs::write(&dart_file, "void main() { print('Goodbye'); }").unwrap();

        let fp2 = compute_dart_fingerprint(
            temp.path(),
            "3.0.0",
            &None,
            &DartProjectType::Dart,
            "native",
            false,
        )
        .unwrap();

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_dart_fingerprint_distinguishes_target_and_mode() {
        let temp = tempdir().unwrap();
        let lib_dir = temp.path().join("lib");
        fs::create_dir_all(&lib_dir).unwrap();
        fs::write(lib_dir.join("main.dart"), "void main() {}").unwrap();
        fs::write(temp.path().join("pubspec.yaml"), "name: test_app").unwrap();

        let debug_native = compute_dart_fingerprint(
            temp.path(),
            "3.0.0",
            &None,
            &DartProjectType::Dart,
            "native",
            false,
        )
        .unwrap();
        let release_native = compute_dart_fingerprint(
            temp.path(),
            "3.0.0",
            &None,
            &DartProjectType::Dart,
            "native",
            true,
        )
        .unwrap();
        let release_apk = compute_dart_fingerprint(
            temp.path(),
            "3.0.0",
            &None,
            &DartProjectType::Flutter,
            "apk",
            true,
        )
        .unwrap();

        assert_ne!(debug_native, release_native);
        assert_ne!(release_native, release_apk);
    }
}
