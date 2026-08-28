use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LanguageKind {
    Rust,
    TypeScript,
    Go,
    Cpp,
    Python,
    Java,
    Dotnet,
    Swift,
    Dart,
    Zig,
    Docker,
    Generic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolVisibility {
    Public,
    Protected,
    Internal,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SymbolKind {
    Function,
    Struct,
    Interface,
    Enum,
    TypeAlias,
    Const,
    Class,
    Module,
    Variable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundarySymbol {
    pub name: String,
    pub kind: SymbolKind,
    pub visibility: SymbolVisibility,
    pub signature: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SymbolicBoundary {
    pub language: LanguageKind,
    pub symbols: Vec<BoundarySymbol>,
    pub boundary_hash: [u8; 32],
}

impl SymbolicBoundary {
    pub fn new(language: LanguageKind, mut symbols: Vec<BoundarySymbol>) -> Self {
        symbols.sort_unstable_by(|a, b| a.name.cmp(&b.name));
        let mut hasher = blake3::Hasher::new();
        hasher.update(&[language as u8]);
        for sym in &symbols {
            if sym.visibility == SymbolVisibility::Public {
                hasher.update(sym.name.as_bytes());
                hasher.update(&[sym.kind as u8]);
                hasher.update(sym.signature.as_bytes());
            }
        }
        let boundary_hash = *hasher.finalize().as_bytes();
        Self {
            language,
            symbols,
            boundary_hash,
        }
    }
}

pub struct PashExtractor;

impl PashExtractor {
    pub fn extract(source: &str, lang: LanguageKind) -> SymbolicBoundary {
        let symbols = match lang {
            LanguageKind::Rust => Self::extract_rust(source),
            LanguageKind::TypeScript => Self::extract_ts(source),
            LanguageKind::Go => Self::extract_go(source),
            LanguageKind::Cpp => Self::extract_cpp(source),
            LanguageKind::Python => Self::extract_python(source),
            _ => Self::extract_generic(source),
        };
        SymbolicBoundary::new(lang, symbols)
    }

    fn extract_rust(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(16);
        for raw_line in source.lines() {
            let line = raw_line.trim();
            let Some(rest) = line.strip_prefix("pub ") else {
                continue;
            };
            if rest.starts_with("fn ")
                || rest.starts_with("async fn ")
                || rest.starts_with("unsafe fn ")
            {
                let sig_end = line.find('{').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = sig
                    .split_whitespace()
                    .find(|w| w.contains('('))
                    .and_then(|w| w.split('(').next())
                    .unwrap_or("unknown")
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::Function,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(body) = rest.strip_prefix("struct ") {
                let sig_end = line
                    .find('{')
                    .or_else(|| line.find(';'))
                    .unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = body
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::Struct,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(body) = rest.strip_prefix("enum ") {
                let sig_end = line.find('{').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = body
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::Enum,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(body) = rest.strip_prefix("trait ") {
                let sig_end = line.find('{').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = body
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::Interface,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(body) = rest.strip_prefix("type ") {
                let sig_end = line.find(';').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = body
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::TypeAlias,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if rest.starts_with("const ") || rest.starts_with("static ") {
                let sig_end = line.find(';').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = sig
                    .split_whitespace()
                    .nth(2)
                    .unwrap_or("unknown")
                    .split(':')
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::Const,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            }
        }
        symbols
    }

    fn extract_ts(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(16);
        for raw_line in source.lines() {
            let line = raw_line.trim();
            let Some(rest) = line.strip_prefix("export ") else {
                continue;
            };
            if rest.starts_with("function ") || rest.starts_with("async function ") {
                let sig_end = line.find('{').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = sig
                    .split_whitespace()
                    .find(|w| w.contains('('))
                    .and_then(|w| w.split('(').next())
                    .unwrap_or("unknown")
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::Function,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(body) = rest.strip_prefix("interface ") {
                let sig_end = line.find('{').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = body
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::Interface,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(body) = rest.strip_prefix("type ") {
                let sig_end = line.find(';').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = body
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .split('=')
                    .next()
                    .unwrap_or("unknown")
                    .trim()
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::TypeAlias,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(body) = rest.strip_prefix("class ") {
                let sig_end = line.find('{').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = body
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::Class,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if rest.starts_with("const ") || rest.starts_with("let ") {
                let sig_end = line
                    .find(';')
                    .or_else(|| line.find('='))
                    .unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = sig
                    .split_whitespace()
                    .nth(2)
                    .unwrap_or("unknown")
                    .split(':')
                    .next()
                    .unwrap_or("unknown")
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::Const,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            }
        }
        symbols
    }

    fn extract_go(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(16);
        for raw_line in source.lines() {
            let line = raw_line.trim();
            if let Some(rest) = line.strip_prefix("func ") {
                let sig_end = line.find('{').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let first_word = rest.split('(').next().unwrap_or("").trim();
                let is_method = rest.starts_with('(');
                let func_name = if is_method {
                    rest.split(')')
                        .nth(1)
                        .unwrap_or("")
                        .trim()
                        .split('(')
                        .next()
                        .unwrap_or("")
                } else {
                    first_word
                };
                if let Some(first_char) = func_name.chars().next() {
                    let visibility = if first_char.is_uppercase() {
                        SymbolVisibility::Public
                    } else {
                        SymbolVisibility::Internal
                    };
                    symbols.push(BoundarySymbol {
                        name: func_name.to_string(),
                        kind: SymbolKind::Function,
                        visibility,
                        signature: sig,
                    });
                }
            } else if let Some(rest) = line.strip_prefix("type ") {
                let parts: Vec<&str> = rest.split_whitespace().collect();
                if parts.len() >= 2 {
                    let name = parts[0];
                    let kind_str = parts[1];
                    let kind = if kind_str.starts_with("struct") {
                        SymbolKind::Struct
                    } else if kind_str.starts_with("interface") {
                        SymbolKind::Interface
                    } else {
                        SymbolKind::TypeAlias
                    };
                    let visibility = if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                        SymbolVisibility::Public
                    } else {
                        SymbolVisibility::Internal
                    };
                    symbols.push(BoundarySymbol {
                        name: name.to_string(),
                        kind,
                        visibility,
                        signature: line.to_string(),
                    });
                }
            }
        }
        symbols
    }

    fn extract_cpp(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(16);
        for raw_line in source.lines() {
            let line = raw_line.trim();
            let is_cpp_decl = line.starts_with("extern \"C\"")
                || line.contains("FISH_EXPORT")
                || line.starts_with("virtual ")
                || line.ends_with(';');
            if is_cpp_decl && line.contains('(') && line.contains(')') && !line.starts_with('#') {
                let sig = line.trim_end_matches(';').trim().to_string();
                let name = sig
                    .split('(')
                    .next()
                    .and_then(|s| s.split_whitespace().last())
                    .unwrap_or("unknown")
                    .trim_start_matches('*')
                    .trim_start_matches('&')
                    .to_string();
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::Function,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            }
        }
        symbols
    }

    fn extract_python(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(16);
        for raw_line in source.lines() {
            let line = raw_line.trim();
            if let Some(stripped) = line.strip_prefix("def ") {
                let name = stripped.split('(').next().unwrap_or("").trim();
                let visibility = if name.starts_with('_') {
                    SymbolVisibility::Internal
                } else {
                    SymbolVisibility::Public
                };
                let sig_end = line.find(':').unwrap_or(line.len());
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Function,
                    visibility,
                    signature: line[..sig_end].to_string(),
                });
            } else if let Some(stripped) = line.strip_prefix("class ") {
                let name = stripped
                    .split('(')
                    .next()
                    .unwrap_or("")
                    .split(':')
                    .next()
                    .unwrap_or("")
                    .trim();
                let visibility = if name.starts_with('_') {
                    SymbolVisibility::Internal
                } else {
                    SymbolVisibility::Public
                };
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Class,
                    visibility,
                    signature: line.to_string(),
                });
            }
        }
        symbols
    }

    fn extract_generic(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(8);
        for (idx, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub ") || trimmed.starts_with("export ") {
                symbols.push(BoundarySymbol {
                    name: format!("symbol_{idx}"),
                    kind: SymbolKind::Module,
                    visibility: SymbolVisibility::Public,
                    signature: trimmed.to_string(),
                });
            }
        }
        symbols
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvalidationDecision {
    Cutoff {
        module_id: String,
        reason: String,
    },
    Cascade {
        module_id: String,
        changed_symbols: Vec<String>,
        affected_downstream: Vec<String>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct PolyAbiHyperGraph {
    boundaries: HashMap<String, SymbolicBoundary>,
    hyperedges: HashMap<String, HashSet<String>>,
}

impl PolyAbiHyperGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_module(&mut self, module_id: &str, lang: LanguageKind, source: &str) {
        let boundary = PashExtractor::extract(source, lang);
        self.boundaries.insert(module_id.to_string(), boundary);
    }

    pub fn add_dependency(&mut self, upstream: &str, downstream: &str) {
        self.hyperedges
            .entry(upstream.to_string())
            .or_default()
            .insert(downstream.to_string());
    }

    pub fn evaluate_diff(
        &mut self,
        module_id: &str,
        lang: LanguageKind,
        new_source: &str,
    ) -> InvalidationDecision {
        let new_boundary = PashExtractor::extract(new_source, lang);
        let downstream_targets = self
            .hyperedges
            .get(module_id)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();

        if let Some(old_boundary) = self.boundaries.get(module_id) {
            if old_boundary.boundary_hash == new_boundary.boundary_hash {
                return InvalidationDecision::Cutoff {
                    module_id: module_id.to_string(),
                    reason: "Public interface boundary invariant holds; downstream polyglot targets preserved".to_string(),
                };
            }

            let old_map: HashMap<&str, &BoundarySymbol> = old_boundary
                .symbols
                .iter()
                .filter(|s| s.visibility == SymbolVisibility::Public)
                .map(|s| (s.name.as_str(), s))
                .collect();

            let mut changed = Vec::new();
            for sym in &new_boundary.symbols {
                if sym.visibility == SymbolVisibility::Public {
                    match old_map.get(sym.name.as_str()) {
                        Some(old_sym) if old_sym.signature != sym.signature => {
                            changed.push(format!("{}: signature modified", sym.name));
                        }
                        None => {
                            changed.push(format!("{}: newly added", sym.name));
                        }
                        _ => {}
                    }
                }
            }

            for old_name in old_map.keys() {
                if !new_boundary
                    .symbols
                    .iter()
                    .any(|s| s.name == *old_name && s.visibility == SymbolVisibility::Public)
                {
                    changed.push(format!("{old_name}: removed"));
                }
            }

            self.boundaries.insert(module_id.to_string(), new_boundary);
            return InvalidationDecision::Cascade {
                module_id: module_id.to_string(),
                changed_symbols: changed,
                affected_downstream: downstream_targets,
            };
        }

        self.boundaries.insert(module_id.to_string(), new_boundary);
        InvalidationDecision::Cascade {
            module_id: module_id.to_string(),
            changed_symbols: vec!["initial registration".to_string()],
            affected_downstream: downstream_targets,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_internal_body_change_produces_cutoff() {
        let mut graph = PolyAbiHyperGraph::new();
        let code_v1 = r#"
pub fn calculate(a: i32, b: i32) -> i32 {
    a + b
}
"#;
        let code_v2 = r#"
pub fn calculate(a: i32, b: i32) -> i32 {
    let result = a + b;
    result * 1
}
"#;
        graph.register_module("rust_core", LanguageKind::Rust, code_v1);
        graph.add_dependency("rust_core", "ts_binding");

        let decision = graph.evaluate_diff("rust_core", LanguageKind::Rust, code_v2);
        match decision {
            InvalidationDecision::Cutoff { reason, .. } => {
                assert!(reason.contains("Public interface boundary invariant holds"));
            }
            InvalidationDecision::Cascade { .. } => panic!("Expected Cutoff"),
        }
    }

    #[test]
    fn test_rust_public_signature_change_produces_cascade() {
        let mut graph = PolyAbiHyperGraph::new();
        let code_v1 = "pub fn calculate(a: i32) -> i32 { a }";
        let code_v2 = "pub fn calculate(a: i32, b: i32) -> i32 { a + b }";

        graph.register_module("rust_core", LanguageKind::Rust, code_v1);
        graph.add_dependency("rust_core", "ts_binding");

        let decision = graph.evaluate_diff("rust_core", LanguageKind::Rust, code_v2);
        match decision {
            InvalidationDecision::Cascade {
                affected_downstream,
                changed_symbols,
                ..
            } => {
                assert_eq!(affected_downstream, vec!["ts_binding"]);
                assert!(!changed_symbols.is_empty());
            }
            InvalidationDecision::Cutoff { .. } => panic!("Expected Cascade"),
        }
    }

    #[test]
    fn test_typescript_export_boundary() {
        let mut graph = PolyAbiHyperGraph::new();
        let ts_v1 = r#"
export function computeTotal(items: number[]): number {
    return items.reduce((a, b) => a + b, 0);
}
"#;
        let ts_v2 = r#"
export function computeTotal(items: number[]): number {
    let sum = 0;
    for (const x of items) { sum += x; }
    return sum;
}
"#;
        graph.register_module("ts_lib", LanguageKind::TypeScript, ts_v1);
        graph.add_dependency("ts_lib", "docker_app");

        let decision = graph.evaluate_diff("ts_lib", LanguageKind::TypeScript, ts_v2);
        match decision {
            InvalidationDecision::Cutoff { .. } => {}
            InvalidationDecision::Cascade { .. } => {
                panic!("Expected Cutoff for internal TS changes")
            }
        }
    }

    #[test]
    fn test_go_private_vs_public_boundary() {
        let mut graph = PolyAbiHyperGraph::new();
        let go_v1 = r#"
package service

func ProcessData(x int) string {
    return helper(x)
}

func helper(x int) string {
    return "ok"
}
"#;
        let go_v2 = r#"
package service

func ProcessData(x int) string {
    return helper(x)
}

func helper(x int) string {
    return "optimized_ok"
}
"#;
        graph.register_module("go_service", LanguageKind::Go, go_v1);
        graph.add_dependency("go_service", "py_worker");

        let decision = graph.evaluate_diff("go_service", LanguageKind::Go, go_v2);
        match decision {
            InvalidationDecision::Cutoff { .. } => {}
            InvalidationDecision::Cascade { .. } => {
                panic!("Expected Cutoff when private Go helper changes")
            }
        }
    }
}
