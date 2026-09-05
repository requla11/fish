use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ZonDependency {
    pub name: String,
    pub url: Option<String>,
    pub hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ZonManifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub dependencies: HashMap<String, ZonDependency>,
    pub paths: Vec<String>,
}

impl ZonManifest {
    pub fn parse(content: &str) -> Self {
        let mut manifest = Self::default();

        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(val) = extract_string_field(trimmed, ".name") {
                manifest.name = Some(val);
            } else if let Some(val) = extract_string_field(trimmed, ".version") {
                manifest.version = Some(val);
            } else if let Some(val) = extract_string_field(trimmed, ".hash") {
                if let Some(dep) = manifest.dependencies.values_mut().last() {
                    dep.hash = Some(val);
                }
            } else if let Some(val) = extract_string_field(trimmed, ".url") {
                if let Some(dep) = manifest.dependencies.values_mut().last() {
                    dep.url = Some(val);
                }
            } else if trimmed.starts_with('.')
                && trimmed.contains("= .{")
                && !trimmed.contains(".dependencies")
                && !trimmed.contains(".paths")
            {
                let dep_name = trimmed
                    .trim_start_matches('.')
                    .split('=')
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !dep_name.is_empty() {
                    manifest.dependencies.insert(
                        dep_name.clone(),
                        ZonDependency {
                            name: dep_name,
                            url: None,
                            hash: None,
                        },
                    );
                }
            }
        }

        manifest
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        Ok(Self::parse(&content))
    }
}

fn extract_string_field(line: &str, field: &str) -> Option<String> {
    if line.starts_with(field) {
        let parts: Vec<&str> = line.split('=').collect();
        if parts.len() >= 2 {
            let val = parts[1].trim().trim_matches(',').trim().trim_matches('"');
            return Some(val.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_build_zig_zon() {
        let sample = r#"
.{
    .name = "my_zig_project",
    .version = "0.2.0",
    .dependencies = .{
        .zap = .{
            .url = "https://github.com/zigzap/zap/archive/v0.1.0.tar.gz",
            .hash = "1220abcdef0123456789",
        },
    },
    .paths = .{
        "build.zig",
        "build.zig.zon",
        "src",
    },
}
"#;
        let manifest = ZonManifest::parse(sample);
        assert_eq!(manifest.name.as_deref(), Some("my_zig_project"));
        assert_eq!(manifest.version.as_deref(), Some("0.2.0"));
        assert!(manifest.dependencies.contains_key("zap"));
        let zap = &manifest.dependencies["zap"];
        assert_eq!(zap.hash.as_deref(), Some("1220abcdef0123456789"));
        assert!(zap.url.as_ref().unwrap().contains("github.com"));
    }
}
