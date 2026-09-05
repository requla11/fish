use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SwiftTarget {
    pub name: String,
    #[serde(rename = "type")]
    pub target_type: Option<String>,
    pub path: Option<String>,
    pub sources: Option<Vec<String>>,
    pub dependencies: Option<Vec<SwiftTargetDependency>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SwiftTargetDependency {
    ByName(String),
    Target { target: Vec<String> },
    Product { product: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SwiftProduct {
    pub name: String,
    #[serde(rename = "type")]
    pub product_type: Option<serde_json::Value>,
    pub targets: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SwiftPackageDescription {
    pub name: String,
    pub targets: Vec<SwiftTarget>,
    pub products: Vec<SwiftProduct>,
}

impl SwiftPackageDescription {
    pub fn parse_json(json_str: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json_str)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let parsed = Self::parse_json(&content)?;
        Ok(parsed)
    }

    pub fn target_names(&self) -> Vec<&str> {
        self.targets.iter().map(|t| t.name.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_swift_package_describe_json() {
        let sample = r#"{
            "name": "MyLibrary",
            "targets": [
                {
                    "name": "MyLibrary",
                    "type": "regular",
                    "path": "Sources/MyLibrary",
                    "sources": ["MyLibrary.swift"]
                },
                {
                    "name": "MyLibraryTests",
                    "type": "test",
                    "path": "Tests/MyLibraryTests",
                    "sources": ["MyLibraryTests.swift"],
                    "dependencies": ["MyLibrary"]
                }
            ],
            "products": [
                {
                    "name": "MyLibrary",
                    "targets": ["MyLibrary"]
                }
            ]
        }"#;

        let pkg = SwiftPackageDescription::parse_json(sample).unwrap();
        assert_eq!(pkg.name, "MyLibrary");
        assert_eq!(pkg.targets.len(), 2);
        assert_eq!(pkg.target_names(), vec!["MyLibrary", "MyLibraryTests"]);
    }
}
