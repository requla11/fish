#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DartProjectType {
    Dart,
    Flutter,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DartTargetPlatform {
    APK,
    IOS,
    Web,
    Windows,
    MacOS,
    Linux,
    All,
}

impl FromStr for DartTargetPlatform {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "apk" => Ok(DartTargetPlatform::APK),
            "ios" => Ok(DartTargetPlatform::IOS),
            "web" => Ok(DartTargetPlatform::Web),
            "windows" => Ok(DartTargetPlatform::Windows),
            "macos" => Ok(DartTargetPlatform::MacOS),
            "linux" => Ok(DartTargetPlatform::Linux),
            "all" => Ok(DartTargetPlatform::All),
            _ => Ok(DartTargetPlatform::All),
        }
    }
}

impl DartTargetPlatform {
    pub fn as_str(&self) -> &str {
        match self {
            DartTargetPlatform::APK => "apk",
            DartTargetPlatform::IOS => "ios",
            DartTargetPlatform::Web => "web",
            DartTargetPlatform::Windows => "windows",
            DartTargetPlatform::MacOS => "macos",
            DartTargetPlatform::Linux => "linux",
            DartTargetPlatform::All => "all",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DartProjectConfig {
    pub project_name: String,
    pub project_type: DartProjectType,
    pub target_platform: DartTargetPlatform,
    pub release: bool,
    pub run_tests: bool,
    pub compile: bool,
    pub is_flutter: bool,
}

impl DartProjectConfig {
    pub fn from_pubspec_yaml(project_dir: &Path) -> Result<Self, String> {
        let pubspec_path = project_dir.join("pubspec.yaml");
        if !pubspec_path.exists() {
            return Err("pubspec.yaml not found".to_string());
        }

        let content = std::fs::read_to_string(&pubspec_path)
            .map_err(|e| format!("Failed to read pubspec.yaml: {}", e))?;

        let project_name =
            Self::extract_project_name(&content).unwrap_or_else(|| "dart_project".to_string());

        let is_flutter = Self::is_flutter_project(&content);
        let project_type = if is_flutter {
            DartProjectType::Flutter
        } else {
            DartProjectType::Dart
        };

        let target_platform = if is_flutter {
            Self::detect_flutter_platform(&content)
        } else {
            DartTargetPlatform::All
        };

        Ok(DartProjectConfig {
            project_name,
            project_type,
            target_platform,
            release: false,
            run_tests: true,
            compile: !is_flutter,
            is_flutter,
        })
    }

    pub fn detect(project_dir: &Path) -> Result<Self, String> {
        if project_dir.join("pubspec.yaml").exists() {
            return Self::from_pubspec_yaml(project_dir);
        }

        Err("No Dart/Flutter project files found (pubspec.yaml)".to_string())
    }

    fn extract_project_name(content: &str) -> Option<String> {
        // Only a top-level `name:` YAML key identifies the package; the old
        // substring search matched `hostname:` and friends anywhere.
        for line in content.lines() {
            let trimmed = line.trim_start();
            if !trimmed.starts_with("name:") {
                continue;
            }
            let value = trimmed["name:".len()..].trim();
            if value.len() >= 2
                && (value.starts_with('"') || value.starts_with('\''))
                && value.ends_with(&value[..1])
            {
                return Some(value[1..value.len() - 1].to_string());
            }
            return Some(value.split_whitespace().next()?.to_string());
        }
        None
    }

    fn is_flutter_project(content: &str) -> bool {
        content.contains("flutter:")
            || content.contains("sdk: flutter")
            || content.contains("flutter_sdk")
    }

    fn detect_flutter_platform(content: &str) -> DartTargetPlatform {
        if content.contains("android") || content.contains("android_intent") {
            return DartTargetPlatform::APK;
        }
        if content.contains("ios") || content.contains("cupertino") {
            return DartTargetPlatform::IOS;
        }
        if content.contains("web") {
            return DartTargetPlatform::Web;
        }
        if content.contains("windows") {
            return DartTargetPlatform::Windows;
        }
        if content.contains("macos") {
            return DartTargetPlatform::MacOS;
        }
        if content.contains("linux") {
            return DartTargetPlatform::Linux;
        }

        DartTargetPlatform::All
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_pubspec_config_detection() {
        let temp = tempdir().unwrap();
        let pubspec_content = r#"
name: test_app
description: A test Dart application
version: 1.0.0

environment:
  sdk: '>=3.0.0 <4.0.0'

dependencies:
  http: ^1.0.0
"#;
        std::fs::write(temp.path().join("pubspec.yaml"), pubspec_content).unwrap();

        let config = DartProjectConfig::from_pubspec_yaml(temp.path()).unwrap();
        assert_eq!(config.project_name, "test_app");
        assert_eq!(config.project_type, DartProjectType::Dart);
        assert!(!config.is_flutter);
    }

    #[test]
    fn test_flutter_project_detection() {
        let temp = tempdir().unwrap();
        let pubspec_content = r#"
name: flutter_app
description: A Flutter application
version: 1.0.0

environment:
  sdk: '>=3.0.0 <4.0.0'

dependencies:
  flutter:
    sdk: flutter
  cupertino_icons: ^1.0.0
"#;
        std::fs::write(temp.path().join("pubspec.yaml"), pubspec_content).unwrap();

        let config = DartProjectConfig::from_pubspec_yaml(temp.path()).unwrap();
        assert_eq!(config.project_name, "flutter_app");
        assert_eq!(config.project_type, DartProjectType::Flutter);
        assert!(config.is_flutter);
    }

    #[test]
    fn test_target_platform_parsing() {
        assert_eq!(
            DartTargetPlatform::from_str("apk").unwrap(),
            DartTargetPlatform::APK
        );
        assert_eq!(
            DartTargetPlatform::from_str("ios").unwrap(),
            DartTargetPlatform::IOS
        );
        assert_eq!(
            DartTargetPlatform::from_str("web").unwrap(),
            DartTargetPlatform::Web
        );
    }
}

#[cfg(test)]
mod name_extraction_tests {
    use super::*;

    #[test]
    fn top_level_name_wins_over_lookalikes() {
        let content = concat!(
            "description: x\nhostname: name: decoy\ndisplay_name: decoy2\n",
            "name: real_app\n"
        );
        assert_eq!(
            DartProjectConfig::extract_project_name(content).as_deref(),
            Some("real_app")
        );
    }

    #[test]
    fn quoted_names_are_unquoted() {
        assert_eq!(
            DartProjectConfig::extract_project_name("name: \"quoted_app\"\n").as_deref(),
            Some("quoted_app")
        );
    }
}
