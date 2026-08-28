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
            LanguageKind::Java => Self::extract_java(source),
            LanguageKind::Dotnet => Self::extract_dotnet(source),
            LanguageKind::Swift => Self::extract_swift(source),
            LanguageKind::Dart => Self::extract_dart(source),
            LanguageKind::Zig => Self::extract_zig(source),
            LanguageKind::Docker => Self::extract_docker(source),
            LanguageKind::Generic => Self::extract_generic(source),
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

    fn extract_java(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(16);
        for raw_line in source.lines() {
            let line = raw_line.trim();
            if !line.starts_with("public ") {
                continue;
            }
            let sig_end = line
                .find('{')
                .or_else(|| line.find(';'))
                .unwrap_or(line.len());
            let sig = line[..sig_end].trim().to_string();
            let after_public = line.strip_prefix("public ").unwrap_or("").trim();

            if let Some(rest) = after_public.strip_prefix("class ") {
                let name = rest.split_whitespace().next().unwrap_or("unknown");
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Class,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(rest) = after_public.strip_prefix("interface ") {
                let name = rest.split_whitespace().next().unwrap_or("unknown");
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Interface,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(rest) = after_public.strip_prefix("enum ") {
                let name = rest.split_whitespace().next().unwrap_or("unknown");
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Enum,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(rest) = after_public.strip_prefix("record ") {
                let name = rest
                    .split('(')
                    .next()
                    .unwrap_or("unknown")
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown");
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Struct,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if sig.contains('(') && sig.contains(')') {
                let name = sig
                    .split('(')
                    .next()
                    .and_then(|s| s.split_whitespace().last())
                    .unwrap_or("unknown");
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Function,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            }
        }
        symbols
    }

    fn extract_dotnet(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(16);
        for raw_line in source.lines() {
            let line = raw_line.trim();
            if !line.starts_with("public ") {
                continue;
            }
            let sig_end = line
                .find('{')
                .or_else(|| line.find(';'))
                .or_else(|| line.find("=>"))
                .unwrap_or(line.len());
            let sig = line[..sig_end].trim().to_string();
            let after_public = line.strip_prefix("public ").unwrap_or("").trim();

            if let Some(rest) = after_public.strip_prefix("class ") {
                let name = rest
                    .split(':')
                    .next()
                    .unwrap_or("unknown")
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown");
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Class,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(rest) = after_public.strip_prefix("interface ") {
                let name = rest
                    .split(':')
                    .next()
                    .unwrap_or("unknown")
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown");
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Interface,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(rest) = after_public.strip_prefix("record ") {
                let name = rest
                    .split('(')
                    .next()
                    .unwrap_or("unknown")
                    .split_whitespace()
                    .next()
                    .unwrap_or("unknown");
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Struct,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(rest) = after_public.strip_prefix("struct ") {
                let name = rest.split_whitespace().next().unwrap_or("unknown");
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Struct,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if let Some(rest) = after_public.strip_prefix("enum ") {
                let name = rest.split_whitespace().next().unwrap_or("unknown");
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Enum,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if sig.contains('(') && sig.contains(')') {
                let name = sig
                    .split('(')
                    .next()
                    .and_then(|s| s.split_whitespace().last())
                    .unwrap_or("unknown");
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Function,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            }
        }
        symbols
    }

    fn extract_swift(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(16);
        for raw_line in source.lines() {
            let line = raw_line.trim();
            let is_public = line.starts_with("public ") || line.starts_with("open ");
            if !is_public {
                continue;
            }
            let sig_end = line.find('{').unwrap_or(line.len());
            let sig = line[..sig_end].trim().to_string();

            if sig.contains("func ") {
                let name = sig
                    .split("func ")
                    .nth(1)
                    .and_then(|s| s.split('(').next())
                    .unwrap_or("unknown")
                    .trim();
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Function,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if sig.contains("protocol ") {
                let name = sig
                    .split("protocol ")
                    .nth(1)
                    .unwrap_or("unknown")
                    .split(':')
                    .next()
                    .unwrap_or("unknown")
                    .trim();
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Interface,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if sig.contains("struct ") || sig.contains("class ") || sig.contains("actor ") {
                let keyword = if sig.contains("struct ") {
                    "struct "
                } else if sig.contains("actor ") {
                    "actor "
                } else {
                    "class "
                };
                let name = sig
                    .split(keyword)
                    .nth(1)
                    .unwrap_or("unknown")
                    .split(':')
                    .next()
                    .unwrap_or("unknown")
                    .trim();
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Class,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if sig.contains("enum ") {
                let name = sig
                    .split("enum ")
                    .nth(1)
                    .unwrap_or("unknown")
                    .split(':')
                    .next()
                    .unwrap_or("unknown")
                    .trim();
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Enum,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            }
        }
        symbols
    }

    fn extract_dart(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(16);
        for raw_line in source.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with("/*") {
                continue;
            }
            if line.starts_with("class ")
                || line.starts_with("abstract class ")
                || line.starts_with("mixin ")
                || line.starts_with("enum ")
            {
                let sig_end = line.find('{').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = sig
                    .split_whitespace()
                    .find(|w| !["class", "abstract", "mixin", "enum"].contains(w))
                    .unwrap_or("unknown")
                    .to_string();
                let visibility = if name.starts_with('_') {
                    SymbolVisibility::Internal
                } else {
                    SymbolVisibility::Public
                };
                symbols.push(BoundarySymbol {
                    name,
                    kind: SymbolKind::Class,
                    visibility,
                    signature: sig,
                });
            } else if line.contains('(')
                && line.contains(')')
                && (line.ends_with('{') || line.ends_with(';'))
            {
                let sig_end = line
                    .find('{')
                    .or_else(|| line.find(';'))
                    .unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = sig
                    .split('(')
                    .next()
                    .and_then(|s| s.split_whitespace().last())
                    .unwrap_or("unknown")
                    .to_string();
                if !name.is_empty()
                    && !name.contains('=')
                    && name != "if"
                    && name != "for"
                    && name != "while"
                {
                    let visibility = if name.starts_with('_') {
                        SymbolVisibility::Internal
                    } else {
                        SymbolVisibility::Public
                    };
                    symbols.push(BoundarySymbol {
                        name,
                        kind: SymbolKind::Function,
                        visibility,
                        signature: sig,
                    });
                }
            }
        }
        symbols
    }

    fn extract_zig(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(16);
        for raw_line in source.lines() {
            let line = raw_line.trim();
            let Some(rest) = line.strip_prefix("pub ") else {
                continue;
            };
            if let Some(func_decl) = rest.strip_prefix("fn ") {
                let sig_end = line.find('{').unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = func_decl.split('(').next().unwrap_or("unknown").trim();
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind: SymbolKind::Function,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            } else if rest.starts_with("const ") || rest.starts_with("var ") {
                let sig_end = line
                    .find('{')
                    .or_else(|| line.find(';'))
                    .unwrap_or(line.len());
                let sig = line[..sig_end].trim().to_string();
                let name = rest
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("unknown")
                    .split(':')
                    .next()
                    .unwrap_or("unknown")
                    .split('=')
                    .next()
                    .unwrap_or("unknown")
                    .trim();
                let kind = if line.contains("struct") {
                    SymbolKind::Struct
                } else if line.contains("enum") {
                    SymbolKind::Enum
                } else {
                    SymbolKind::Const
                };
                symbols.push(BoundarySymbol {
                    name: name.to_string(),
                    kind,
                    visibility: SymbolVisibility::Public,
                    signature: sig,
                });
            }
        }
        symbols
    }

    fn extract_docker(source: &str) -> Vec<BoundarySymbol> {
        let mut symbols = Vec::with_capacity(8);
        for raw_line in source.lines() {
            let line = raw_line.trim();
            if line.starts_with("EXPOSE ")
                || line.starts_with("ENTRYPOINT ")
                || line.starts_with("CMD ")
                || line.starts_with("VOLUME ")
                || line.starts_with("ENV ")
            {
                let first_word = line.split_whitespace().next().unwrap_or("DOCKER");
                symbols.push(BoundarySymbol {
                    name: format!("directive_{first_word}"),
                    kind: SymbolKind::Module,
                    visibility: SymbolVisibility::Public,
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
            if trimmed.starts_with("pub ")
                || trimmed.starts_with("export ")
                || trimmed.starts_with("public ")
            {
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
    fn test_java_internal_change_produces_cutoff() {
        let mut graph = PolyAbiHyperGraph::new();
        let java_v1 = r#"
package com.example;

public class OrderService {
    public int calculateTotal(int price, int quantity) {
        return price * quantity;
    }
    private void auditLog() {
        System.out.println("log v1");
    }
}
"#;
        let java_v2 = r#"
package com.example;

public class OrderService {
    public int calculateTotal(int price, int quantity) {
        int total = price * quantity;
        return total;
    }
    private void auditLog() {
        System.out.println("log v2 optimized");
    }
}
"#;
        graph.register_module("java_service", LanguageKind::Java, java_v1);
        graph.add_dependency("java_service", "go_gateway");

        let decision = graph.evaluate_diff("java_service", LanguageKind::Java, java_v2);
        match decision {
            InvalidationDecision::Cutoff { .. } => {}
            InvalidationDecision::Cascade { .. } => {
                panic!("Expected Cutoff for Java internal logic change")
            }
        }
    }

    #[test]
    fn test_dotnet_record_and_method_cascade() {
        let mut graph = PolyAbiHyperGraph::new();
        let cs_v1 = "public record UserDto(string Name);";
        let cs_v2 = "public record UserDto(string Name, int Age);";

        graph.register_module("dotnet_api", LanguageKind::Dotnet, cs_v1);
        graph.add_dependency("dotnet_api", "web_client");

        let decision = graph.evaluate_diff("dotnet_api", LanguageKind::Dotnet, cs_v2);
        match decision {
            InvalidationDecision::Cascade {
                affected_downstream,
                ..
            } => {
                assert_eq!(affected_downstream, vec!["web_client"]);
            }
            InvalidationDecision::Cutoff { .. } => {
                panic!("Expected Cascade on C# signature change")
            }
        }
    }

    #[test]
    fn test_swift_open_func_cutoff() {
        let mut graph = PolyAbiHyperGraph::new();
        let swift_v1 = r#"
open class NetworkClient {
    open func sendRequest(url: String) -> String {
        return "data_v1"
    }
}
"#;
        let swift_v2 = r#"
open class NetworkClient {
    open func sendRequest(url: String) -> String {
        let cached = "data_v1"
        return cached
    }
}
"#;
        graph.register_module("swift_sdk", LanguageKind::Swift, swift_v1);
        graph.add_dependency("swift_sdk", "flutter_ui");

        let decision = graph.evaluate_diff("swift_sdk", LanguageKind::Swift, swift_v2);
        match decision {
            InvalidationDecision::Cutoff { .. } => {}
            InvalidationDecision::Cascade { .. } => panic!("Expected Cutoff for Swift body edit"),
        }
    }

    #[test]
    fn test_zig_pub_fn_and_docker_invariants() {
        let mut graph = PolyAbiHyperGraph::new();
        let zig_v1 = r#"
pub fn add(a: i32, b: i32) i32 {
    return a + b;
}
"#;
        let zig_v2 = r#"
pub fn add(a: i32, b: i32) i32 {
    const res = a + b;
    return res;
}
"#;
        graph.register_module("zig_math", LanguageKind::Zig, zig_v1);
        graph.add_dependency("zig_math", "c_app");

        let decision = graph.evaluate_diff("zig_math", LanguageKind::Zig, zig_v2);
        match decision {
            InvalidationDecision::Cutoff { .. } => {}
            InvalidationDecision::Cascade { .. } => {
                panic!("Expected Cutoff for Zig internal body edit")
            }
        }
    }
}
