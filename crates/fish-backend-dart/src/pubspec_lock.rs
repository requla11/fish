use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PubspecPackage {
    pub name: String,
    pub version: String,
    pub source: String,
    pub description_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PubspecLock {
    pub packages: HashMap<String, PubspecPackage>,
}

impl PubspecLock {
    pub fn parse(content: &str) -> Self {
        let mut lock = Self::default();
        let mut current_pkg: Option<PubspecPackage> = None;
        let mut in_packages_section = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "packages:" {
                in_packages_section = true;
                continue;
            }

            if in_packages_section {
                if !line.starts_with("  ") && !trimmed.is_empty() {
                    if let Some(pkg) = current_pkg.take() {
                        lock.packages.insert(pkg.name.clone(), pkg);
                    }
                    in_packages_section = false;
                    continue;
                }

                if line.starts_with("  ") && !line.starts_with("    ") && trimmed.ends_with(':') {
                    if let Some(pkg) = current_pkg.take() {
                        lock.packages.insert(pkg.name.clone(), pkg);
                    }
                    let pkg_name = trimmed.trim_end_matches(':').to_string();
                    current_pkg = Some(PubspecPackage {
                        name: pkg_name,
                        ..Default::default()
                    });
                } else if line.starts_with("    ")
                    && let Some(pkg) = &mut current_pkg
                {
                    if trimmed.starts_with("version:") {
                        let ver = trimmed
                            .trim_start_matches("version:")
                            .trim()
                            .trim_matches('"');
                        pkg.version = ver.to_string();
                    } else if trimmed.starts_with("source:") {
                        let src = trimmed
                            .trim_start_matches("source:")
                            .trim()
                            .trim_matches('"');
                        pkg.source = src.to_string();
                    } else if trimmed.starts_with("url:") {
                        let u = trimmed.trim_start_matches("url:").trim().trim_matches('"');
                        pkg.description_url = Some(u.to_string());
                    }
                }
            }
        }

        if let Some(pkg) = current_pkg {
            lock.packages.insert(pkg.name.clone(), pkg);
        }

        lock
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::parse(&content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_pubspec_lock() {
        let sample = r#"
packages:
  async:
    dependency: "direct main"
    description:
      name: async
      sha256: "9476161405e3f2d96202c43e37b20f8b1fe62e2303c6a300bfda82c21306915d"
      url: "https://pub.dev"
    source: hosted
    version: "2.11.0"
  path:
    dependency: "transitive"
    description:
      name: path
      sha256: "087ce89c2193ea3dd342d22b7430ec07e7d405bfda11856233c3745e63d4b7b8"
      url: "https://pub.dev"
    source: hosted
    version: "1.9.0"
sdks:
  dart: ">=3.0.0 <4.0.0"
"#;

        let lock = PubspecLock::parse(sample);
        assert_eq!(lock.packages.len(), 2);
        assert!(lock.packages.contains_key("async"));
        assert_eq!(lock.packages["async"].version, "2.11.0");
        assert_eq!(lock.packages["async"].source, "hosted");
        assert_eq!(lock.packages["path"].version, "1.9.0");
    }
}
