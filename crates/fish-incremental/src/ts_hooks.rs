use std::path::{Path, PathBuf};
use std::collections::{HashMap, BTreeSet};
use serde::{Deserialize, Serialize};

use crate::compiler_hooks::{ItemKind, ChangeKind, ItemDiff, DiffResult, RebuildDecision, SemanticItem, ModuleSnapshot};
use crate::semantic_impact::SemanticImpactGraph;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TsDialect {
    TypeScript,
    JavaScript,
    Tsx,
    Jsx,
}

impl TsDialect {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "ts" => Some(Self::TypeScript),
            "tsx" => Some(Self::Tsx),
            "js" => Some(Self::JavaScript),
            "jsx" => Some(Self::Jsx),
            "mts" | "cts" => Some(Self::TypeScript),
            "mjs" | "cjs" => Some(Self::JavaScript),
            _ => None,
        }
    }

    pub fn supports_types(self) -> bool {
        matches!(self, Self::TypeScript | Self::Tsx)
    }
}

struct TsParser<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> TsParser<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
        }
    }

    fn remaining(&self) -> &'a str {
        &self.source[self.pos..]
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn skip_line_comment(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
            self.pos += 1;
        }
    }

    fn skip_block_comment(&mut self) {
        self.pos += 2;
        while self.pos + 1 < self.bytes.len() {
            if self.bytes[self.pos] == b'*' && self.bytes[self.pos + 1] == b'/' {
                self.pos += 2;
                return;
            }
            self.pos += 1;
        }
        self.pos = self.bytes.len();
    }

    fn skip_string(&mut self, quote: u8) {
        self.pos += 1;
        while self.pos < self.bytes.len() {
            if self.bytes[self.pos] == b'\\' {
                self.pos += 2;
                continue;
            }
            if self.bytes[self.pos] == quote {
                self.pos += 1;
                return;
            }
            self.pos += 1;
        }
    }

    fn skip_template_literal(&mut self) {
        self.pos += 1;
        let mut depth = 0u32;
        while self.pos < self.bytes.len() {
            if self.bytes[self.pos] == b'\\' {
                self.pos += 2;
                continue;
            }
            if self.bytes[self.pos] == b'$'
                && self.pos + 1 < self.bytes.len()
                && self.bytes[self.pos + 1] == b'{'
            {
                depth += 1;
                self.pos += 2;
                continue;
            }
            if self.bytes[self.pos] == b'}' && depth > 0 {
                depth -= 1;
                self.pos += 1;
                continue;
            }
            if self.bytes[self.pos] == b'`' && depth == 0 {
                self.pos += 1;
                return;
            }
            self.pos += 1;
        }
    }

    fn read_identifier(&mut self) -> Option<String> {
        self.skip_whitespace();
        let start = self.pos;
        while self.pos < self.bytes.len()
            && (self.bytes[self.pos].is_ascii_alphanumeric()
                || self.bytes[self.pos] == b'_'
                || self.bytes[self.pos] == b'$')
        {
            self.pos += 1;
        }
        if self.pos > start {
            Some(self.source[start..self.pos].to_string())
        } else {
            None
        }
    }

    fn find_matching_brace(&mut self) -> usize {
        let start = self.pos;
        let mut depth = 0i32;
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'{' => depth += 1,
                b'}' => {
                    depth -= 1;
                    if depth == 0 {
                        self.pos += 1;
                        return self.pos;
                    }
                }
                b'\'' | b'"' => self.skip_string(self.bytes[self.pos]),
                b'`' => self.skip_template_literal(),
                b'/' if self.pos + 1 < self.bytes.len() => {
                    if self.bytes[self.pos + 1] == b'/' {
                        self.skip_line_comment();
                    } else if self.bytes[self.pos + 1] == b'*' {
                        self.skip_block_comment();
                    } else {
                        self.pos += 1;
                    }
                    continue;
                }
                _ => {}
            }
            self.pos += 1;
        }
        start
    }

    fn consume_to_end_of_declaration(&mut self) -> usize {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'{' => return self.find_matching_brace(),
                b';' | b'\n' => {
                    self.pos += 1;
                    return self.pos;
                }
                b'\'' | b'"' => self.skip_string(self.bytes[self.pos]),
                b'`' => self.skip_template_literal(),
                b'(' => {
                    let mut depth = 1i32;
                    self.pos += 1;
                    while self.pos < self.bytes.len() && depth > 0 {
                        match self.bytes[self.pos] {
                            b'(' => depth += 1,
                            b')' => depth -= 1,
                            b'\'' | b'"' => { self.skip_string(self.bytes[self.pos]); continue; }
                            _ => {}
                        }
                        self.pos += 1;
                    }
                    continue;
                }
                _ => { self.pos += 1; continue; }
            }
            self.pos += 1;
        }
        self.pos
    }
}

#[derive(Debug, Clone)]
struct TsItem {
    name: String,
    kind: ItemKind,
    body_start: usize,
    body_end: usize,
    is_exported: bool,
}

pub fn parse_ts_module(file_path: &Path, content: &str) -> Result<ModuleSnapshot, String> {
    let items = extract_ts_items(content);

    let content_no_comments = strip_ts_comments(content);
    let module_hash = blake3::hash(content_no_comments.as_bytes()).to_hex().to_string();

    let mut semantic_items = Vec::new();
    let all_names: std::collections::HashSet<String> = items.iter().map(|i| i.name.clone()).collect();

    for item in &items {
        let body = &content[item.body_start..item.body_end];
        let body_no_comments = strip_ts_comments(body);
        let canonical_hash = blake3::hash(body_no_comments.as_bytes()).to_hex().to_string();

        let sig = extract_ts_signature(body, &item.kind);
        let sig_no_comments = strip_ts_comments(&sig);
        let signature_hash = blake3::hash(sig_no_comments.as_bytes()).to_hex().to_string();

        let visibility = if item.is_exported { "export".to_string() } else { String::new() };

        semantic_items.push(SemanticItem {
            name: item.name.clone(),
            kind: item.kind.clone(),
            canonical_hash,
            signature_hash,
            visibility,
        });
    }

    let mut edges: HashMap<String, BTreeSet<String>> = HashMap::new();
    for item in &items {
        let body = &content[item.body_start..item.body_end];
        let mut refs = BTreeSet::new();
        for other_name in &all_names {
            if *other_name != item.name && body.contains(other_name.as_str()) {
                refs.insert(other_name.clone());
            }
        }
        if !refs.is_empty() {
            edges.insert(item.name.clone(), refs);
        }
    }

    Ok(ModuleSnapshot {
        file_path: file_path.to_path_buf(),
        module_hash,
        items: semantic_items,
        edges,
    })
}

fn extract_ts_items(source: &str) -> Vec<TsItem> {
    let mut items = Vec::new();
    let mut parser = TsParser::new(source);

    while parser.pos < parser.bytes.len() {
        parser.skip_whitespace();
        if parser.pos >= parser.bytes.len() {
            break;
        }

        if parser.pos + 1 < parser.bytes.len()
            && parser.bytes[parser.pos] == b'/'
            && parser.bytes[parser.pos + 1] == b'/'
        {
            parser.skip_line_comment();
            continue;
        }
        if parser.pos + 1 < parser.bytes.len()
            && parser.bytes[parser.pos] == b'/'
            && parser.bytes[parser.pos + 1] == b'*'
        {
            parser.skip_block_comment();
            continue;
        }

        let line_start = parser.pos;
        let remaining = parser.remaining();

        let is_exported = remaining.starts_with("export ");
        let effective = if is_exported { &remaining[7..] } else { remaining };

        let effective_start = if is_exported { parser.pos + 7 } else { parser.pos };

        if let Some(item) = try_parse_ts_declaration(effective, effective_start, is_exported, &mut parser) {
            items.push(item);
        } else {
            while parser.pos < parser.bytes.len()
                && parser.bytes[parser.pos] != b'\n'
                && parser.bytes[parser.pos] != b';'
            {
                parser.pos += 1;
            }
            if parser.pos < parser.bytes.len() {
                parser.pos += 1;
            }
            if parser.pos == line_start {
                parser.pos += 1;
            }
        }
    }

    items
}

fn try_parse_ts_declaration(
    text: &str,
    abs_offset: usize,
    is_exported: bool,
    parser: &mut TsParser<'_>,
) -> Option<TsItem> {
    let keywords = [
        ("function ", ItemKind::Function),
        ("async function ", ItemKind::Function),
        ("class ", ItemKind::Struct),
        ("interface ", ItemKind::Trait),
        ("type ", ItemKind::TypeAlias),
        ("enum ", ItemKind::Enum),
        ("const ", ItemKind::Const),
        ("let ", ItemKind::Static),
        ("var ", ItemKind::Static),
    ];

    for (keyword, kind) in &keywords {
        if text.starts_with(keyword) {
            let after_kw = &text[keyword.len()..];
            let name_end = after_kw
                .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '$')
                .unwrap_or(after_kw.len());

            if name_end == 0 {
                continue;
            }

            let name = after_kw[..name_end].to_string();
            let body_start = if is_exported {
                abs_offset - 7
            } else {
                abs_offset
            };

            parser.pos = abs_offset + keyword.len() + name_end;
            let body_end = parser.consume_to_end_of_declaration();

            return Some(TsItem {
                name,
                kind: kind.clone(),
                body_start,
                body_end,
                is_exported,
            });
        }
    }

    None
}

fn strip_ts_comments(source: &str) -> String {
    let bytes = source.as_bytes();
    let mut result = String::with_capacity(source.len());
    let mut i = 0;

    while i < bytes.len() {
        if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
        } else if i + 1 < bytes.len() && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < bytes.len() {
                i += 2;
            }
        } else if bytes[i] == b'\'' || bytes[i] == b'"' {
            let quote = bytes[i];
            result.push(bytes[i] as char);
            i += 1;
            while i < bytes.len() {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    result.push(bytes[i] as char);
                    result.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                result.push(bytes[i] as char);
                if bytes[i] == quote {
                    i += 1;
                    break;
                }
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

fn extract_ts_signature(body: &str, kind: &ItemKind) -> String {
    match kind {
        ItemKind::Function => {
            if let Some(brace_pos) = body.find('{') {
                body[..brace_pos].trim().to_string()
            } else {
                body.lines().next().unwrap_or("").to_string()
            }
        }
        ItemKind::Struct | ItemKind::Trait => {
            if let Some(brace_pos) = body.find('{') {
                body[..brace_pos].trim().to_string()
            } else {
                body.lines().next().unwrap_or("").to_string()
            }
        }
        ItemKind::TypeAlias => {
            body.lines().next().unwrap_or("").to_string()
        }
        ItemKind::Enum => {
            if let Some(brace_pos) = body.find('{') {
                body[..brace_pos].trim().to_string()
            } else {
                body.lines().next().unwrap_or("").to_string()
            }
        }
        _ => {
            let first_line = body.lines().next().unwrap_or("");
            if let Some(eq_pos) = first_line.find('=') {
                first_line[..eq_pos].trim().to_string()
            } else {
                first_line.to_string()
            }
        }
    }
}

pub fn diff_ts_modules(old: &ModuleSnapshot, new: &ModuleSnapshot) -> DiffResult {
    crate::compiler_hooks::diff_snapshots(old, new)
}

pub fn compute_ts_rebuild_decision(
    diff: &DiffResult,
    snapshot: &ModuleSnapshot,
    impact_graph: Option<&SemanticImpactGraph>,
) -> RebuildDecision {
    crate::compiler_hooks::compute_rebuild_decision(diff, snapshot, impact_graph)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ts_dialect_detection() {
        assert_eq!(TsDialect::from_extension("ts"), Some(TsDialect::TypeScript));
        assert_eq!(TsDialect::from_extension("tsx"), Some(TsDialect::Tsx));
        assert_eq!(TsDialect::from_extension("js"), Some(TsDialect::JavaScript));
        assert!(TsDialect::from_extension("rs").is_none());
        assert!(TsDialect::TypeScript.supports_types());
        assert!(!TsDialect::JavaScript.supports_types());
    }

    #[test]
    fn test_ts_basic_function_extraction() {
        let source = r#"
export function greet(name: string): string {
    return `Hello, ${name}`;
}

function internal(x: number): number {
    return x * 2;
}
"#;
        let snapshot = parse_ts_module(Path::new("app.ts"), source).unwrap();
        let names: Vec<&str> = snapshot.items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"greet"));
        assert!(names.contains(&"internal"));

        let greet = snapshot.items.iter().find(|i| i.name == "greet").unwrap();
        assert_eq!(greet.visibility, "export");
        assert_eq!(greet.kind, ItemKind::Function);
    }

    #[test]
    fn test_ts_class_interface_enum() {
        let source = r#"
export class UserService {
    private db: Database;
    constructor(db: Database) { this.db = db; }
    async findUser(id: string): Promise<User> { return this.db.find(id); }
}

export interface Config {
    port: number;
    host: string;
}

export enum Status {
    Active,
    Inactive,
    Pending,
}
"#;
        let snapshot = parse_ts_module(Path::new("service.ts"), source).unwrap();
        let names: Vec<&str> = snapshot.items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"UserService"));
        assert!(names.contains(&"Config"));
        assert!(names.contains(&"Status"));
    }

    #[test]
    fn test_ts_const_arrow_function() {
        let source = r#"
export const API_URL = "https://api.example.com";
const helper = (x: number) => x + 1;
export const compute = (a: number, b: number): number => {
    return helper(a) + helper(b);
};
"#;
        let snapshot = parse_ts_module(Path::new("config.ts"), source).unwrap();
        let names: Vec<&str> = snapshot.items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"API_URL"));
        assert!(names.contains(&"helper"));
        assert!(names.contains(&"compute"));
    }

    #[test]
    fn test_ts_comment_stripping_preserves_hash_stability() {
        let src1 = "export function add(a: number, b: number): number { return a + b; }";
        let src2 = r#"
// Adds two numbers together
/** @param a first number */
export function add(a: number, b: number): number {
    return a + b; // sum
}
"#;
        let s1 = parse_ts_module(Path::new("a.ts"), src1).unwrap();
        let s2 = parse_ts_module(Path::new("b.ts"), src2).unwrap();

        let add1 = s1.items.iter().find(|i| i.name == "add").unwrap();
        let add2 = s2.items.iter().find(|i| i.name == "add").unwrap();
        assert_eq!(add1.signature_hash, add2.signature_hash);
    }

    #[test]
    fn test_ts_diff_body_change() {
        let old_src = "export function calc(x: number): number { return x + 1; }";
        let new_src = "export function calc(x: number): number { return x + 999; }";

        let old = parse_ts_module(Path::new("a.ts"), old_src).unwrap();
        let new = parse_ts_module(Path::new("a.ts"), new_src).unwrap();
        let diff = diff_ts_modules(&old, &new);

        assert!(diff.module_hash_changed);
        assert_eq!(diff.diffs.len(), 1);
        assert_eq!(diff.diffs[0].name, "calc");
        assert_eq!(diff.diffs[0].change, ChangeKind::BodyModified);
    }

    #[test]
    fn test_ts_diff_signature_change() {
        let old_src = "export function calc(x: number): number { return x; }";
        let new_src = "export function calc(x: number, y: number): number { return x + y; }";

        let old = parse_ts_module(Path::new("a.ts"), old_src).unwrap();
        let new = parse_ts_module(Path::new("a.ts"), new_src).unwrap();
        let diff = diff_ts_modules(&old, &new);

        assert_eq!(diff.diffs[0].change, ChangeKind::SignatureModified);
    }

    #[test]
    fn test_ts_dependency_edges() {
        let source = r#"
function base(): number { return 42; }
function caller(): number { return base() + 1; }
function standalone(): void { console.log("hi"); }
"#;
        let snap = parse_ts_module(Path::new("deps.ts"), source).unwrap();
        let caller_deps = snap.edges.get("caller");
        assert!(caller_deps.is_some());
        assert!(caller_deps.unwrap().contains("base"));
        assert!(snap.edges.get("standalone").is_none());
    }

    #[test]
    fn test_ts_cascade_decision() {
        let old_src = r#"
function base(): number { return 1; }
function middle(): number { return base() + 1; }
function top(): number { return middle(); }
function isolated(): void {}
"#;
        let new_src = r#"
function base(): [number, number] { return [1, 2]; }
function middle(): number { return base() + 1; }
function top(): number { return middle(); }
function isolated(): void {}
"#;
        let old = parse_ts_module(Path::new("c.ts"), old_src).unwrap();
        let new = parse_ts_module(Path::new("c.ts"), new_src).unwrap();
        let diff = diff_ts_modules(&old, &new);
        let decision = compute_ts_rebuild_decision(&diff, &new, None);

        assert!(decision.must_rebuild.contains(&"base".to_string()));
        assert!(decision.must_rebuild.contains(&"middle".to_string()));
        assert!(decision.safe_to_skip.contains(&"isolated".to_string()));
    }

    #[test]
    fn test_ts_async_function() {
        let source = "export async function fetchData(url: string): Promise<Response> { return await fetch(url); }";
        let snap = parse_ts_module(Path::new("api.ts"), source).unwrap();
        assert!(snap.items.iter().any(|i| i.name == "fetchData" && i.kind == ItemKind::Function));
    }

    #[test]
    fn test_ts_type_alias() {
        let source = "export type UserId = string;\nexport type Config = { port: number; host: string };";
        let snap = parse_ts_module(Path::new("types.ts"), source).unwrap();
        let names: Vec<&str> = snap.items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"UserId"));
        assert!(names.contains(&"Config"));
    }
}
