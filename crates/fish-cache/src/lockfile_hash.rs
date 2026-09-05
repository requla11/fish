use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockfileKind {
    CargoLock,
    NpmLock,
    PnpmLock,
    YarnLock,
    BunLock,
    PoetryLock,
    GoSum,
    Generic,
}

pub struct LockfileHasher;

impl LockfileHasher {
    pub fn detect_kind(path: &Path) -> LockfileKind {
        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or_default();

        match filename {
            "Cargo.lock" => LockfileKind::CargoLock,
            "package-lock.json" => LockfileKind::NpmLock,
            "pnpm-lock.yaml" => LockfileKind::PnpmLock,
            "yarn.lock" => LockfileKind::YarnLock,
            "bun.lock" | "bun.lockb" => LockfileKind::BunLock,
            "poetry.lock" => LockfileKind::PoetryLock,
            "go.sum" => LockfileKind::GoSum,
            _ => LockfileKind::Generic,
        }
    }

    pub fn compute_canonical_hash(path: &Path, content: &[u8]) -> String {
        match Self::detect_kind(path) {
            LockfileKind::CargoLock => Self::hash_cargo_lock(content),
            LockfileKind::GoSum => Self::hash_go_sum(content),
            LockfileKind::NpmLock => Self::hash_npm_lock(content),
            _ => Self::hash_generic(content),
        }
    }

    fn hash_generic(content: &[u8]) -> String {
        let text = String::from_utf8_lossy(content);
        let mut normalized = String::with_capacity(text.len());
        for line in text.lines() {
            let trimmed = line.trim_end();
            if !trimmed.is_empty() {
                normalized.push_str(trimmed);
                normalized.push('\n');
            }
        }
        blake3::hash(normalized.as_bytes()).to_hex().to_string()
    }

    fn hash_go_sum(content: &[u8]) -> String {
        let text = String::from_utf8_lossy(content);
        let mut lines: Vec<&str> = text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        lines.sort_unstable();

        let mut hasher = blake3::Hasher::new();
        for line in lines {
            hasher.update(line.as_bytes());
            hasher.update(b"\n");
        }
        hasher.finalize().to_hex().to_string()
    }

    fn hash_cargo_lock(content: &[u8]) -> String {
        let text = String::from_utf8_lossy(content);
        let mut packages = Vec::new();

        let mut current_name = String::new();
        let mut current_version = String::new();
        let mut current_checksum = String::new();
        let mut in_package = false;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed == "[[package]]" {
                if in_package && !current_name.is_empty() {
                    packages.push((
                        std::mem::take(&mut current_name),
                        std::mem::take(&mut current_version),
                        std::mem::take(&mut current_checksum),
                    ));
                }
                in_package = true;
                continue;
            }

            if in_package {
                if let Some(rest) = trimmed.strip_prefix("name = \"")
                    && let Some(val) = rest.strip_suffix('"')
                {
                    current_name = val.to_string();
                } else if let Some(rest) = trimmed.strip_prefix("version = \"")
                    && let Some(val) = rest.strip_suffix('"')
                {
                    current_version = val.to_string();
                } else if let Some(rest) = trimmed.strip_prefix("checksum = \"")
                    && let Some(val) = rest.strip_suffix('"')
                {
                    current_checksum = val.to_string();
                }
            }
        }

        if in_package && !current_name.is_empty() {
            packages.push((current_name, current_version, current_checksum));
        }

        if packages.is_empty() {
            return Self::hash_generic(content);
        }

        packages.sort();

        let mut hasher = blake3::Hasher::new();
        for (name, version, checksum) in packages {
            hasher.update(name.as_bytes());
            hasher.update(b"@");
            hasher.update(version.as_bytes());
            hasher.update(b"#");
            hasher.update(checksum.as_bytes());
            hasher.update(b"\n");
        }
        hasher.finalize().to_hex().to_string()
    }

    fn hash_npm_lock(content: &[u8]) -> String {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(content)
            && let Some(packages) = val.get("packages").and_then(|p| p.as_object())
        {
            let mut entries = Vec::new();
            for (pkg_path, pkg_info) in packages {
                let version = pkg_info
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();
                let integrity = pkg_info
                    .get("integrity")
                    .and_then(|i| i.as_str())
                    .unwrap_or_default();
                entries.push((pkg_path.clone(), version.to_string(), integrity.to_string()));
            }
            entries.sort();

            let mut hasher = blake3::Hasher::new();
            for (path, ver, integ) in entries {
                hasher.update(path.as_bytes());
                hasher.update(b":");
                hasher.update(ver.as_bytes());
                hasher.update(b":");
                hasher.update(integ.as_bytes());
                hasher.update(b"\n");
            }
            return hasher.finalize().to_hex().to_string();
        }
        Self::hash_generic(content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generic_crlf_lf_equivalence() {
        let lf_content = b"line1\nline2\nline3\n";
        let crlf_content = b"line1\r\nline2\r\nline3\r\n";

        let hash1 = LockfileHasher::compute_canonical_hash(Path::new("yarn.lock"), lf_content);
        let hash2 = LockfileHasher::compute_canonical_hash(Path::new("yarn.lock"), crlf_content);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_go_sum_ordering_independence() {
        let c1 = b"github.com/foo/bar v1.0.0 h1:abc\ngithub.com/baz/qux v2.0.0 h1:def\n";
        let c2 = b"github.com/baz/qux v2.0.0 h1:def\r\ngithub.com/foo/bar v1.0.0 h1:abc\r\n";

        let hash1 = LockfileHasher::compute_canonical_hash(Path::new("go.sum"), c1);
        let hash2 = LockfileHasher::compute_canonical_hash(Path::new("go.sum"), c2);

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_cargo_lock_package_order_independence() {
        let p1 = r#"
[[package]]
name = "alpha"
version = "1.0.0"
checksum = "1111"

[[package]]
name = "beta"
version = "2.0.0"
checksum = "2222"
"#;

        let p2 = r#"
[[package]]
name = "beta"
version = "2.0.0"
checksum = "2222"

[[package]]
name = "alpha"
version = "1.0.0"
checksum = "1111"
"#;

        let hash1 = LockfileHasher::compute_canonical_hash(Path::new("Cargo.lock"), p1.as_bytes());
        let hash2 = LockfileHasher::compute_canonical_hash(Path::new("Cargo.lock"), p2.as_bytes());

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_npm_package_lock_json_normalization() {
        let j1 = r#"{
  "name": "my-app",
  "packages": {
    "node_modules/a": { "version": "1.0.0", "integrity": "sha512-aaa" },
    "node_modules/b": { "version": "2.0.0", "integrity": "sha512-bbb" }
  }
}"#;

        let j2 = r#"{
  "name": "my-app",
  "packages": {
    "node_modules/b": { "version": "2.0.0", "integrity": "sha512-bbb" },
    "node_modules/a": { "version": "1.0.0", "integrity": "sha512-aaa" }
  }
}"#;

        let hash1 =
            LockfileHasher::compute_canonical_hash(Path::new("package-lock.json"), j1.as_bytes());
        let hash2 =
            LockfileHasher::compute_canonical_hash(Path::new("package-lock.json"), j2.as_bytes());

        assert_eq!(hash1, hash2);
    }
}
