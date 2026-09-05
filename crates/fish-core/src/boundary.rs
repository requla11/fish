use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BoundaryViolationKind {
    DisallowedDependency,
    DeniedDependency,
    DisallowedDependent,
    DeniedDependent,
}

impl fmt::Display for BoundaryViolationKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DisallowedDependency => write!(f, "DisallowedDependency"),
            Self::DeniedDependency => write!(f, "DeniedDependency"),
            Self::DisallowedDependent => write!(f, "DisallowedDependent"),
            Self::DeniedDependent => write!(f, "DeniedDependent"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundaryViolation {
    pub violator_package: String,
    pub target_package: String,
    pub violator_tag: String,
    pub target_tag: Option<String>,
    pub kind: BoundaryViolationKind,
}

impl fmt::Display for BoundaryViolation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] package '{}' (tag '{}') violates architectural boundary with package '{}' (tag '{:?}')",
            self.kind,
            self.violator_package,
            self.violator_tag,
            self.target_package,
            self.target_tag
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct BoundaryTagRule {
    pub tag: String,
    pub allow_dependencies: Option<Vec<String>>,
    pub deny_dependencies: Option<Vec<String>>,
    pub allow_dependents: Option<Vec<String>>,
    pub deny_dependents: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageBoundaryMeta {
    pub name: String,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct BoundaryEnforcer {
    rules: HashMap<String, BoundaryTagRule>,
}

impl BoundaryEnforcer {
    pub fn new(rules: Vec<BoundaryTagRule>) -> Self {
        let mut map = HashMap::with_capacity(rules.len());
        for rule in rules {
            map.insert(rule.tag.clone(), rule);
        }
        Self { rules: map }
    }

    pub fn add_rule(&mut self, rule: BoundaryTagRule) {
        self.rules.insert(rule.tag.clone(), rule);
    }

    pub fn check(&self, packages: &[PackageBoundaryMeta]) -> Result<(), Vec<BoundaryViolation>> {
        let mut package_map: HashMap<&str, &PackageBoundaryMeta> =
            HashMap::with_capacity(packages.len());
        for pkg in packages {
            package_map.insert(&pkg.name, pkg);
        }

        let mut violations = Vec::new();

        for pkg in packages {
            for dep_name in &pkg.dependencies {
                let Some(dep_pkg) = package_map.get(dep_name.as_str()) else {
                    continue;
                };

                for from_tag in &pkg.tags {
                    if let Some(rule) = self.rules.get(from_tag) {
                        if let Some(denied) = &rule.deny_dependencies {
                            for target_tag in &dep_pkg.tags {
                                if denied.contains(target_tag) {
                                    violations.push(BoundaryViolation {
                                        violator_package: pkg.name.clone(),
                                        target_package: dep_pkg.name.clone(),
                                        violator_tag: from_tag.clone(),
                                        target_tag: Some(target_tag.clone()),
                                        kind: BoundaryViolationKind::DeniedDependency,
                                    });
                                }
                            }
                        }

                        if let Some(allowed) = &rule.allow_dependencies
                            && !dep_pkg.tags.is_empty()
                        {
                            let has_allowed_tag = dep_pkg
                                .tags
                                .iter()
                                .any(|target_tag| allowed.contains(target_tag));
                            if !has_allowed_tag {
                                violations.push(BoundaryViolation {
                                    violator_package: pkg.name.clone(),
                                    target_package: dep_pkg.name.clone(),
                                    violator_tag: from_tag.clone(),
                                    target_tag: dep_pkg.tags.first().cloned(),
                                    kind: BoundaryViolationKind::DisallowedDependency,
                                });
                            }
                        }
                    }
                }

                for to_tag in &dep_pkg.tags {
                    if let Some(target_rule) = self.rules.get(to_tag) {
                        if let Some(denied_dependents) = &target_rule.deny_dependents {
                            for from_tag in &pkg.tags {
                                if denied_dependents.contains(from_tag) {
                                    violations.push(BoundaryViolation {
                                        violator_package: pkg.name.clone(),
                                        target_package: dep_pkg.name.clone(),
                                        violator_tag: from_tag.clone(),
                                        target_tag: Some(to_tag.clone()),
                                        kind: BoundaryViolationKind::DeniedDependent,
                                    });
                                }
                            }
                        }

                        if let Some(allowed_dependents) = &target_rule.allow_dependents
                            && !pkg.tags.is_empty()
                        {
                            let has_allowed = pkg
                                .tags
                                .iter()
                                .any(|from_tag| allowed_dependents.contains(from_tag));
                            if !has_allowed {
                                violations.push(BoundaryViolation {
                                    violator_package: pkg.name.clone(),
                                    target_package: dep_pkg.name.clone(),
                                    violator_tag: pkg.tags.first().cloned().unwrap_or_default(),
                                    target_tag: Some(to_tag.clone()),
                                    kind: BoundaryViolationKind::DisallowedDependent,
                                });
                            }
                        }
                    }
                }
            }
        }

        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boundary_deny_dependency() {
        let rule = BoundaryTagRule {
            tag: "ui".to_string(),
            allow_dependencies: None,
            deny_dependencies: Some(vec!["database".to_string()]),
            allow_dependents: None,
            deny_dependents: None,
        };

        let enforcer = BoundaryEnforcer::new(vec![rule]);

        let pkgs = vec![
            PackageBoundaryMeta {
                name: "web-client".to_string(),
                tags: vec!["ui".to_string()],
                dependencies: vec!["db-driver".to_string()],
            },
            PackageBoundaryMeta {
                name: "db-driver".to_string(),
                tags: vec!["database".to_string()],
                dependencies: Vec::new(),
            },
        ];

        let result = enforcer.check(&pkgs);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].violator_package, "web-client");
        assert_eq!(violations[0].target_package, "db-driver");
        assert_eq!(violations[0].kind, BoundaryViolationKind::DeniedDependency);
    }

    #[test]
    fn test_boundary_allow_dependency_success() {
        let rule = BoundaryTagRule {
            tag: "ui".to_string(),
            allow_dependencies: Some(vec!["shared".to_string()]),
            deny_dependencies: None,
            allow_dependents: None,
            deny_dependents: None,
        };

        let enforcer = BoundaryEnforcer::new(vec![rule]);

        let pkgs = vec![
            PackageBoundaryMeta {
                name: "web-client".to_string(),
                tags: vec!["ui".to_string()],
                dependencies: vec!["shared-components".to_string()],
            },
            PackageBoundaryMeta {
                name: "shared-components".to_string(),
                tags: vec!["shared".to_string()],
                dependencies: Vec::new(),
            },
        ];

        assert!(enforcer.check(&pkgs).is_ok());
    }

    #[test]
    fn test_boundary_deny_dependents() {
        let rule = BoundaryTagRule {
            tag: "internal_crypto".to_string(),
            allow_dependencies: None,
            deny_dependencies: None,
            allow_dependents: Some(vec!["core_engine".to_string()]),
            deny_dependents: None,
        };

        let enforcer = BoundaryEnforcer::new(vec![rule]);

        let pkgs = vec![
            PackageBoundaryMeta {
                name: "third_party_plugin".to_string(),
                tags: vec!["plugin".to_string()],
                dependencies: vec!["crypto_vault".to_string()],
            },
            PackageBoundaryMeta {
                name: "crypto_vault".to_string(),
                tags: vec!["internal_crypto".to_string()],
                dependencies: Vec::new(),
            },
        ];

        let result = enforcer.check(&pkgs);
        assert!(result.is_err());
        let violations = result.unwrap_err();
        assert_eq!(violations.len(), 1);
        assert_eq!(
            violations[0].kind,
            BoundaryViolationKind::DisallowedDependent
        );
    }
}
