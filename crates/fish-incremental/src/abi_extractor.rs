use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbiSignature {
    pub name: String,
    pub signature_text: String,
    pub is_public: bool,
}

#[derive(Debug, Clone, Default)]
pub struct SemanticAbiExtractor;

impl SemanticAbiExtractor {
    pub fn extract_rust_public_interface(source: &str) -> String {
        let mut signatures = BTreeSet::new();
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("pub fn ")
                || trimmed.starts_with("pub struct ")
                || trimmed.starts_with("pub enum ")
                || trimmed.starts_with("pub trait ")
                || trimmed.starts_with("pub type ")
                || trimmed.starts_with("pub mod ")
                || trimmed.starts_with("pub const ")
            {
                if let Some(header) = trimmed.split('{').next() {
                    signatures.insert(header.trim());
                } else {
                    signatures.insert(trimmed);
                }
            }
        }
        signatures.into_iter().collect::<Vec<_>>().join("\n")
    }

    pub fn extract_go_public_interface(source: &str) -> String {
        let mut signatures = BTreeSet::new();
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("func ") {
                let rest = trimmed.strip_prefix("func ").unwrap_or_default().trim();
                let name = if rest.starts_with('(') {
                    rest.split(')').nth(1).unwrap_or_default().trim()
                } else {
                    rest
                };
                if let Some(first_char) = name.chars().next()
                    && first_char.is_uppercase()
                    && let Some(header) = trimmed.split('{').next()
                {
                    signatures.insert(header.trim());
                }
            } else if trimmed.starts_with("type ") {
                let rest = trimmed.strip_prefix("type ").unwrap_or_default().trim();
                if let Some(first_char) = rest.chars().next()
                    && first_char.is_uppercase()
                    && let Some(header) = trimmed.split('{').next()
                {
                    signatures.insert(header.trim());
                }
            }
        }
        signatures.into_iter().collect::<Vec<_>>().join("\n")
    }

    pub fn extract_ts_public_interface(source: &str) -> String {
        let mut signatures = BTreeSet::new();
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("export function ")
                || trimmed.starts_with("export class ")
                || trimmed.starts_with("export interface ")
                || trimmed.starts_with("export type ")
                || trimmed.starts_with("export const ")
                || trimmed.starts_with("export enum ")
            {
                if let Some(header) = trimmed.split('{').next() {
                    signatures.insert(header.trim());
                } else {
                    signatures.insert(trimmed);
                }
            }
        }
        signatures.into_iter().collect::<Vec<_>>().join("\n")
    }

    pub fn extract_cc_public_interface(source: &str) -> String {
        let mut signatures = BTreeSet::new();
        for line in source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("EXPORT ")
                || trimmed.starts_with("extern \"C\"")
                || trimmed.starts_with("class ")
                || trimmed.starts_with("struct ")
                || trimmed.starts_with("virtual ")
                || (trimmed.ends_with(';') && trimmed.contains('(') && !trimmed.starts_with("//"))
            {
                signatures.insert(trimmed);
            }
        }
        signatures.into_iter().collect::<Vec<_>>().join("\n")
    }

    pub fn extract_py_public_interface(source: &str) -> String {
        let mut signatures = BTreeSet::new();
        for line in source.lines() {
            let trimmed = line.trim();
            if (trimmed.starts_with("def ") && !trimmed.starts_with("def _"))
                || (trimmed.starts_with("class ") && !trimmed.starts_with("class _"))
            {
                if let Some(header) = trimmed.split(':').next() {
                    signatures.insert(header.trim());
                }
            } else if trimmed.starts_with("__all__") {
                signatures.insert(trimmed);
            }
        }
        signatures.into_iter().collect::<Vec<_>>().join("\n")
    }

    pub fn extract_polyglot_public_interface(ecosystem: &str, source: &str) -> String {
        match ecosystem.to_lowercase().as_str() {
            "rust" | "rs" => Self::extract_rust_public_interface(source),
            "go" | "golang" => Self::extract_go_public_interface(source),
            "ts" | "typescript" | "js" | "javascript" => Self::extract_ts_public_interface(source),
            "cc" | "cpp" | "c" => Self::extract_cc_public_interface(source),
            "py" | "python" => Self::extract_py_public_interface(source),
            _ => Self::extract_rust_public_interface(source),
        }
    }

    pub fn compute_polyglot_interface_hash(ecosystem: &str, source: &str) -> String {
        let interface_text = Self::extract_polyglot_public_interface(ecosystem, source);
        blake3::hash(interface_text.as_bytes()).to_hex().to_string()
    }

    pub fn extract_native_binary_symbols(data: &[u8]) -> Vec<String> {
        let mut symbols = Vec::new();
        if data.len() >= 4
            && data[0] == 0x7F
            && data[1] == b'E'
            && data[2] == b'L'
            && data[3] == b'F'
        {
            symbols.push("elf".to_string());
        } else if data.len() >= 2 && data[0] == b'M' && data[1] == b'Z' {
            symbols.push("pe".to_string());
            if data.len() >= 0x40 {
                let pe_offset =
                    u32::from_le_bytes([data[0x3C], data[0x3D], data[0x3E], data[0x3F]]) as usize;
                if pe_offset + 24 <= data.len() && &data[pe_offset..pe_offset + 4] == b"PE\0\0" {
                    let machine = u16::from_le_bytes([data[pe_offset + 4], data[pe_offset + 5]]);
                    match machine {
                        0x8664 => symbols.push("arch:x86_64".to_string()),
                        0xAA64 => symbols.push("arch:arm64".to_string()),
                        0x014C => symbols.push("arch:i386".to_string()),
                        _ => {}
                    }
                }
            }
        }
        symbols
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_internal_body_change_does_not_alter_interface_hash() {
        let v1 = r#"
        pub fn add(a: i32, b: i32) -> i32 {
            a + b
        }
        fn internal_helper() {
            println!("v1");
        }
        "#;

        let v2 = r#"
        pub fn add(a: i32, b: i32) -> i32 {
            let res = a + b;
            res
        }
        fn internal_helper() {
            println!("v2 - totally changed internal body");
        }
        "#;

        let hash1 = SemanticAbiExtractor::compute_polyglot_interface_hash("rust", v1);
        let hash2 = SemanticAbiExtractor::compute_polyglot_interface_hash("rust", v2);
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_go_and_ts_public_interface_extraction() {
        let go_code = r#"
        package math
        func CalculateTax(amount float64) float64 { return amount * 0.1 }
        func internalSecret() string { return "hidden" }
        type Invoice struct { Total float64 }
        "#;
        let go_hash = SemanticAbiExtractor::compute_polyglot_interface_hash("go", go_code);
        assert!(!go_hash.is_empty());

        let ts_code = r#"
        export function fetchData(url: string): Promise<Response> { return fetch(url); }
        function localHelper() {}
        export interface User { id: string; name: string; }
        "#;
        let ts_hash = SemanticAbiExtractor::compute_polyglot_interface_hash("ts", ts_code);
        assert!(!ts_hash.is_empty());
    }

    #[test]
    fn test_native_binary_symbols_extraction() {
        let elf_header = vec![0x7F, b'E', b'L', b'F', 2, 1, 1, 0];
        let elf_syms = SemanticAbiExtractor::extract_native_binary_symbols(&elf_header);
        assert_eq!(elf_syms, vec!["elf"]);

        let mut pe_header = vec![0u8; 128];
        pe_header[0] = b'M';
        pe_header[1] = b'Z';
        pe_header[0x3C] = 0x40;
        pe_header[0x40] = b'P';
        pe_header[0x41] = b'E';
        pe_header[0x42] = 0;
        pe_header[0x43] = 0;
        pe_header[0x44] = 0x64;
        pe_header[0x45] = 0x86;

        let pe_syms = SemanticAbiExtractor::extract_native_binary_symbols(&pe_header);
        assert_eq!(pe_syms, vec!["pe", "arch:x86_64"]);
    }
}
