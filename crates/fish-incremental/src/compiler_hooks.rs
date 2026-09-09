use std::path::{Path, PathBuf};
use std::collections::{HashMap, HashSet, BTreeSet};
use quote::ToTokens;
use syn::visit::Visit;
use syn::{Item, File, Ident};
use serde::{Deserialize, Serialize};

use crate::ast_cache::AstSubTree;
use crate::semantic_impact::SemanticImpactGraph;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemKind {
    Function,
    Struct,
    Enum,
    Trait,
    Impl,
    TypeAlias,
    Const,
    Static,
    Mod,
    Macro,
    Use,
}

impl std::fmt::Display for ItemKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Function => write!(f, "fn"),
            Self::Struct => write!(f, "struct"),
            Self::Enum => write!(f, "enum"),
            Self::Trait => write!(f, "trait"),
            Self::Impl => write!(f, "impl"),
            Self::TypeAlias => write!(f, "type"),
            Self::Const => write!(f, "const"),
            Self::Static => write!(f, "static"),
            Self::Mod => write!(f, "mod"),
            Self::Macro => write!(f, "macro"),
            Self::Use => write!(f, "use"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticItem {
    pub name: String,
    pub kind: ItemKind,
    pub canonical_hash: String,
    pub signature_hash: String,
    pub visibility: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleSnapshot {
    pub file_path: PathBuf,
    pub module_hash: String,
    pub items: Vec<SemanticItem>,
    pub edges: HashMap<String, BTreeSet<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeKind {
    Added,
    Removed,
    BodyModified,
    SignatureModified,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemDiff {
    pub name: String,
    pub kind: ItemKind,
    pub change: ChangeKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    pub file_path: PathBuf,
    pub diffs: Vec<ItemDiff>,
    pub module_hash_changed: bool,
}

impl DiffResult {
    pub fn has_signature_changes(&self) -> bool {
        self.diffs.iter().any(|d| d.change == ChangeKind::SignatureModified
            || d.change == ChangeKind::Added
            || d.change == ChangeKind::Removed)
    }

    pub fn changed_item_names(&self) -> Vec<String> {
        self.diffs.iter().map(|d| d.name.clone()).collect()
    }

    pub fn is_clean(&self) -> bool {
        self.diffs.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebuildDecision {
    pub must_rebuild: Vec<String>,
    pub safe_to_skip: Vec<String>,
    pub cascade_targets: Vec<String>,
    pub affected_tests: Vec<String>,
    pub reuse_ratio: f64,
}

struct IdentCollector {
    referenced: HashSet<String>,
    defined: HashSet<String>,
}

impl IdentCollector {
    fn new() -> Self {
        Self {
            referenced: HashSet::new(),
            defined: HashSet::new(),
        }
    }
}

impl<'ast> Visit<'ast> for IdentCollector {
    fn visit_ident(&mut self, ident: &'ast Ident) {
        self.referenced.insert(ident.to_string());
    }
}

fn extract_item_name(item: &Item) -> Option<(String, ItemKind)> {
    match item {
        Item::Fn(i) => Some((i.sig.ident.to_string(), ItemKind::Function)),
        Item::Struct(i) => Some((i.ident.to_string(), ItemKind::Struct)),
        Item::Enum(i) => Some((i.ident.to_string(), ItemKind::Enum)),
        Item::Trait(i) => Some((i.ident.to_string(), ItemKind::Trait)),
        Item::Impl(i) => {
            let target = i.self_ty.to_token_stream().to_string().replace(' ', "");
            let name = if let Some((_, trait_path, _)) = &i.trait_ {
                format!("impl_{}_for_{}", trait_path.to_token_stream().to_string().replace(' ', ""), target)
            } else {
                format!("impl_{target}")
            };
            Some((name, ItemKind::Impl))
        }
        Item::Type(i) => Some((i.ident.to_string(), ItemKind::TypeAlias)),
        Item::Const(i) => Some((i.ident.to_string(), ItemKind::Const)),
        Item::Static(i) => Some((i.ident.to_string(), ItemKind::Static)),
        Item::Mod(i) => Some((i.ident.to_string(), ItemKind::Mod)),
        Item::Macro(i) => i.ident.as_ref().map(|id| (id.to_string(), ItemKind::Macro)),
        _ => None,
    }
}

fn extract_visibility(item: &Item) -> String {
    match item {
        Item::Fn(i) => i.vis.to_token_stream().to_string(),
        Item::Struct(i) => i.vis.to_token_stream().to_string(),
        Item::Enum(i) => i.vis.to_token_stream().to_string(),
        Item::Trait(i) => i.vis.to_token_stream().to_string(),
        Item::Type(i) => i.vis.to_token_stream().to_string(),
        Item::Const(i) => i.vis.to_token_stream().to_string(),
        Item::Static(i) => i.vis.to_token_stream().to_string(),
        Item::Mod(i) => i.vis.to_token_stream().to_string(),
        _ => String::new(),
    }
}

fn extract_signature_hash(item: &Item) -> String {
    let sig_tokens = match item {
        Item::Fn(i) => i.sig.to_token_stream().to_string(),
        Item::Struct(i) => {
            let vis = i.vis.to_token_stream().to_string();
            let name = i.ident.to_string();
            let generics = i.generics.to_token_stream().to_string();
            let fields = i.fields.to_token_stream().to_string();
            format!("{vis} struct {name}{generics} {fields}")
        }
        Item::Enum(i) => {
            let vis = i.vis.to_token_stream().to_string();
            let name = i.ident.to_string();
            let generics = i.generics.to_token_stream().to_string();
            let variants = i.variants.to_token_stream().to_string();
            format!("{vis} enum {name}{generics} {{ {variants} }}")
        }
        Item::Trait(i) => {
            let vis = i.vis.to_token_stream().to_string();
            let name = i.ident.to_string();
            let generics = i.generics.to_token_stream().to_string();
            let bounds = i.supertraits.to_token_stream().to_string();
            format!("{vis} trait {name}{generics}: {bounds}")
        }
        Item::Type(i) => i.to_token_stream().to_string(),
        Item::Const(i) => {
            let vis = i.vis.to_token_stream().to_string();
            let name = i.ident.to_string();
            let ty = i.ty.to_token_stream().to_string();
            format!("{vis} const {name}: {ty}")
        }
        Item::Static(i) => {
            let vis = i.vis.to_token_stream().to_string();
            let name = i.ident.to_string();
            let ty = i.ty.to_token_stream().to_string();
            format!("{vis} static {name}: {ty}")
        }
        _ => item.to_token_stream().to_string(),
    };
    blake3::hash(sig_tokens.as_bytes()).to_hex().to_string()
}

fn collect_references_from_item(item: &Item, known_symbols: &HashSet<String>) -> BTreeSet<String> {
    let mut collector = IdentCollector::new();
    syn::visit::visit_item(&mut collector, item);

    collector.referenced
        .into_iter()
        .filter(|name| known_symbols.contains(name))
        .collect()
}

pub fn parse_module(file_path: &Path, content: &str) -> Result<ModuleSnapshot, String> {
    let syntax_tree: File = syn::parse_file(content)
        .map_err(|e| format!("failed to parse Rust AST for {}: {e}", file_path.display()))?;

    let canonical_module_code = syntax_tree.to_token_stream().to_string();
    let module_hash = blake3::hash(canonical_module_code.as_bytes()).to_hex().to_string();

    let mut items = Vec::new();
    let mut name_to_item: HashMap<String, &Item> = HashMap::new();
    let mut all_names: HashSet<String> = HashSet::new();

    for item in &syntax_tree.items {
        if let Some((name, kind)) = extract_item_name(item) {
            let canonical = item.to_token_stream().to_string();
            let canonical_hash = blake3::hash(canonical.as_bytes()).to_hex().to_string();
            let signature_hash = extract_signature_hash(item);
            let visibility = extract_visibility(item);

            items.push(SemanticItem {
                name: name.clone(),
                kind,
                canonical_hash,
                signature_hash,
                visibility,
            });

            name_to_item.insert(name.clone(), item);
            all_names.insert(name);
        }
    }

    let mut edges: HashMap<String, BTreeSet<String>> = HashMap::new();
    for (name, item) in &name_to_item {
        let mut refs = collect_references_from_item(item, &all_names);
        refs.remove(name);
        if !refs.is_empty() {
            edges.insert(name.clone(), refs);
        }
    }

    Ok(ModuleSnapshot {
        file_path: file_path.to_path_buf(),
        module_hash,
        items,
        edges,
    })
}

pub fn diff_snapshots(old: &ModuleSnapshot, new: &ModuleSnapshot) -> DiffResult {
    let module_hash_changed = old.module_hash != new.module_hash;

    let old_map: HashMap<&str, &SemanticItem> = old.items.iter()
        .map(|i| (i.name.as_str(), i))
        .collect();
    let new_map: HashMap<&str, &SemanticItem> = new.items.iter()
        .map(|i| (i.name.as_str(), i))
        .collect();

    let mut diffs = Vec::new();

    for new_item in &new.items {
        match old_map.get(new_item.name.as_str()) {
            None => {
                diffs.push(ItemDiff {
                    name: new_item.name.clone(),
                    kind: new_item.kind.clone(),
                    change: ChangeKind::Added,
                });
            }
            Some(old_item) => {
                if old_item.signature_hash != new_item.signature_hash {
                    diffs.push(ItemDiff {
                        name: new_item.name.clone(),
                        kind: new_item.kind.clone(),
                        change: ChangeKind::SignatureModified,
                    });
                } else if old_item.canonical_hash != new_item.canonical_hash {
                    diffs.push(ItemDiff {
                        name: new_item.name.clone(),
                        kind: new_item.kind.clone(),
                        change: ChangeKind::BodyModified,
                    });
                }
            }
        }
    }

    for old_item in &old.items {
        if !new_map.contains_key(old_item.name.as_str()) {
            diffs.push(ItemDiff {
                name: old_item.name.clone(),
                kind: old_item.kind.clone(),
                change: ChangeKind::Removed,
            });
        }
    }

    DiffResult {
        file_path: new.file_path.clone(),
        diffs,
        module_hash_changed,
    }
}

fn transitive_dependents(
    item_name: &str,
    edges: &HashMap<String, BTreeSet<String>>,
) -> HashSet<String> {
    let mut reverse: HashMap<&str, Vec<&str>> = HashMap::new();
    for (from, targets) in edges {
        for to in targets {
            reverse.entry(to.as_str()).or_default().push(from.as_str());
        }
    }

    let mut visited = HashSet::new();
    let mut queue = vec![item_name];
    while let Some(current) = queue.pop() {
        if !visited.insert(current.to_string()) {
            continue;
        }
        if let Some(dependents) = reverse.get(current) {
            for dep in dependents {
                queue.push(dep);
            }
        }
    }
    visited.remove(item_name);
    visited
}

pub fn compute_rebuild_decision(
    diff: &DiffResult,
    snapshot: &ModuleSnapshot,
    impact_graph: Option<&SemanticImpactGraph>,
) -> RebuildDecision {
    let changed_names: HashSet<String> = diff.diffs.iter().map(|d| d.name.clone()).collect();
    let all_names: HashSet<String> = snapshot.items.iter().map(|i| i.name.clone()).collect();

    let mut must_rebuild: HashSet<String> = changed_names.clone();
    let mut cascade_targets: HashSet<String> = HashSet::new();

    for diff_item in &diff.diffs {
        if diff_item.change == ChangeKind::SignatureModified
            || diff_item.change == ChangeKind::Removed
        {
            let dependents = transitive_dependents(&diff_item.name, &snapshot.edges);
            for dep in &dependents {
                cascade_targets.insert(dep.clone());
                must_rebuild.insert(dep.clone());
            }
        }
    }

    let safe_to_skip: Vec<String> = all_names
        .difference(&must_rebuild)
        .cloned()
        .collect();

    let total = all_names.len();
    let reuse_ratio = if total == 0 {
        1.0
    } else {
        safe_to_skip.len() as f64 / total as f64
    };

    let affected_tests = if let Some(graph) = impact_graph {
        let modified_symbols: Vec<String> = must_rebuild.iter().cloned().collect();
        graph.find_impacted_tests(&modified_symbols)
    } else {
        Vec::new()
    };

    RebuildDecision {
        must_rebuild: must_rebuild.into_iter().collect(),
        safe_to_skip,
        cascade_targets: cascade_targets.into_iter().collect(),
        affected_tests,
        reuse_ratio,
    }
}

pub fn snapshot_to_ast_subtrees(snapshot: &ModuleSnapshot) -> Vec<AstSubTree> {
    snapshot.items.iter().map(|item| {
        AstSubTree {
            symbol_name: item.name.clone(),
            kind: item.kind.to_string(),
            content_hash: item.canonical_hash.clone(),
            byte_range: (0, 0),
        }
    }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        write!(f, "{content}").unwrap();
        f
    }

    #[test]
    fn test_ast_hashing_ignores_comments_and_whitespace() {
        let f1 = write_temp("pub struct Config { pub count: usize }\nfn work() { println!(\"hello\"); }");
        let f2 = write_temp("/// doc comment\npub struct Config {\n    pub count: usize, // inline\n}\n\n// line comment\nfn work() {\n    println!(\"hello\");\n}");

        let s1 = parse_module(f1.path(), &std::fs::read_to_string(f1.path()).unwrap()).unwrap();
        let s2 = parse_module(f2.path(), &std::fs::read_to_string(f2.path()).unwrap()).unwrap();

        assert_eq!(s1.module_hash, s2.module_hash);
        let s1_cfg = s1.items.iter().find(|i| i.name == "Config").unwrap();
        let s2_cfg = s2.items.iter().find(|i| i.name == "Config").unwrap();
        assert_eq!(s1_cfg.canonical_hash, s2_cfg.canonical_hash);
    }

    #[test]
    fn test_body_change_detected_signature_stable() {
        let f_old = write_temp("fn compute(x: i32) -> i32 { x + 1 }");
        let f_new = write_temp("fn compute(x: i32) -> i32 { x + 999 }");

        let old = parse_module(f_old.path(), &std::fs::read_to_string(f_old.path()).unwrap()).unwrap();
        let new = parse_module(f_new.path(), &std::fs::read_to_string(f_new.path()).unwrap()).unwrap();
        let diff = diff_snapshots(&old, &new);

        assert!(diff.module_hash_changed);
        assert_eq!(diff.diffs.len(), 1);
        assert_eq!(diff.diffs[0].name, "compute");
        assert_eq!(diff.diffs[0].change, ChangeKind::BodyModified);
    }

    #[test]
    fn test_signature_change_detected() {
        let f_old = write_temp("fn compute(x: i32) -> i32 { x }");
        let f_new = write_temp("fn compute(x: i32, y: i32) -> i32 { x + y }");

        let old = parse_module(f_old.path(), &std::fs::read_to_string(f_old.path()).unwrap()).unwrap();
        let new = parse_module(f_new.path(), &std::fs::read_to_string(f_new.path()).unwrap()).unwrap();
        let diff = diff_snapshots(&old, &new);

        assert!(diff.has_signature_changes());
        assert_eq!(diff.diffs[0].change, ChangeKind::SignatureModified);
    }

    #[test]
    fn test_added_and_removed_items() {
        let f_old = write_temp("fn alpha() {} fn beta() {}");
        let f_new = write_temp("fn alpha() {} fn gamma() {}");

        let old = parse_module(f_old.path(), &std::fs::read_to_string(f_old.path()).unwrap()).unwrap();
        let new = parse_module(f_new.path(), &std::fs::read_to_string(f_new.path()).unwrap()).unwrap();
        let diff = diff_snapshots(&old, &new);

        let names: HashSet<String> = diff.diffs.iter().map(|d| d.name.clone()).collect();
        assert!(names.contains("beta"));
        assert!(names.contains("gamma"));

        let beta_diff = diff.diffs.iter().find(|d| d.name == "beta").unwrap();
        assert_eq!(beta_diff.change, ChangeKind::Removed);

        let gamma_diff = diff.diffs.iter().find(|d| d.name == "gamma").unwrap();
        assert_eq!(gamma_diff.change, ChangeKind::Added);
    }

    #[test]
    fn test_intra_module_dependency_extraction() {
        let source = r#"
            struct Config { value: usize }
            fn get_config() -> Config { Config { value: 42 } }
            fn use_config() { let c = get_config(); }
            fn standalone() { println!("no deps"); }
        "#;
        let f = write_temp(source);
        let snap = parse_module(f.path(), source).unwrap();

        let use_config_deps = snap.edges.get("use_config");
        assert!(use_config_deps.is_some());
        let deps = use_config_deps.unwrap();
        assert!(deps.contains("get_config"));
    }

    #[test]
    fn test_transitive_cascade_on_signature_change() {
        let old_src = r#"
            fn base() -> i32 { 42 }
            fn middle() -> i32 { base() + 1 }
            fn top() -> i32 { middle() + 1 }
            fn isolated() -> i32 { 999 }
        "#;
        let new_src = r#"
            fn base() -> (i32, i32) { (42, 0) }
            fn middle() -> i32 { base() + 1 }
            fn top() -> i32 { middle() + 1 }
            fn isolated() -> i32 { 999 }
        "#;

        let f_old = write_temp(old_src);
        let f_new = write_temp(new_src);
        let old = parse_module(f_old.path(), old_src).unwrap();
        let new = parse_module(f_new.path(), new_src).unwrap();

        let diff = diff_snapshots(&old, &new);
        let decision = compute_rebuild_decision(&diff, &new, None);

        assert!(decision.must_rebuild.contains(&"base".to_string()));
        assert!(decision.must_rebuild.contains(&"middle".to_string()));
        assert!(decision.safe_to_skip.contains(&"isolated".to_string()));
        assert!(decision.reuse_ratio > 0.0);
    }

    #[test]
    fn test_body_only_change_no_cascade() {
        let old_src = "fn leaf() -> i32 { 1 }\nfn caller() -> i32 { leaf() }";
        let new_src = "fn leaf() -> i32 { 999 }\nfn caller() -> i32 { leaf() }";

        let f_old = write_temp(old_src);
        let f_new = write_temp(new_src);
        let old = parse_module(f_old.path(), old_src).unwrap();
        let new = parse_module(f_new.path(), new_src).unwrap();

        let diff = diff_snapshots(&old, &new);
        assert_eq!(diff.diffs.len(), 1);
        assert_eq!(diff.diffs[0].change, ChangeKind::BodyModified);

        let decision = compute_rebuild_decision(&diff, &new, None);
        assert!(decision.must_rebuild.contains(&"leaf".to_string()));
        assert!(decision.safe_to_skip.contains(&"caller".to_string()));
        assert!(decision.cascade_targets.is_empty());
    }

    #[test]
    fn test_semantic_impact_integration() {
        let old_src = "fn calculate_tax(x: f64) -> f64 { x * 0.1 }";
        let new_src = "fn calculate_tax(x: f64) -> f64 { x * 0.2 }";

        let f_old = write_temp(old_src);
        let f_new = write_temp(new_src);
        let old = parse_module(f_old.path(), old_src).unwrap();
        let new = parse_module(f_new.path(), new_src).unwrap();

        let mut impact = SemanticImpactGraph::new();
        impact.map_symbol_to_test("calculate_tax", "tests::tax_10_percent");
        impact.map_symbol_to_test("calculate_tax", "tests::tax_invoice_total");

        let diff = diff_snapshots(&old, &new);
        let decision = compute_rebuild_decision(&diff, &new, Some(&impact));

        assert_eq!(decision.affected_tests.len(), 2);
        assert!(decision.affected_tests.contains(&"tests::tax_10_percent".to_string()));
        assert!(decision.affected_tests.contains(&"tests::tax_invoice_total".to_string()));
    }

    #[test]
    fn test_no_change_full_reuse() {
        let src = "fn stable() -> i32 { 42 }\nstruct Point { x: f64, y: f64 }";
        let f1 = write_temp(src);
        let f2 = write_temp(src);
        let s1 = parse_module(f1.path(), src).unwrap();
        let s2 = parse_module(f2.path(), src).unwrap();

        let diff = diff_snapshots(&s1, &s2);
        assert!(diff.is_clean());
        assert!(!diff.module_hash_changed);

        let decision = compute_rebuild_decision(&diff, &s2, None);
        assert!((decision.reuse_ratio - 1.0).abs() < 1e-9);
        assert!(decision.must_rebuild.is_empty());
    }

    #[test]
    fn test_impl_block_tracking() {
        let src = r#"
            struct Foo;
            impl Foo { fn bar(&self) -> i32 { 1 } }
            trait Baz { fn qux(&self); }
            impl Baz for Foo { fn qux(&self) {} }
        "#;
        let f = write_temp(src);
        let snap = parse_module(f.path(), src).unwrap();

        let names: Vec<&str> = snap.items.iter().map(|i| i.name.as_str()).collect();
        assert!(names.contains(&"Foo"));
        assert!(names.iter().any(|n| n.starts_with("impl_Foo")));
        assert!(names.iter().any(|n| n.contains("Baz") && n.contains("Foo")));
    }

    #[test]
    fn test_snapshot_to_ast_subtrees_bridge() {
        let src = "fn alpha() {}\nstruct Beta { x: i32 }";
        let f = write_temp(src);
        let snap = parse_module(f.path(), src).unwrap();

        let subtrees = snapshot_to_ast_subtrees(&snap);
        assert_eq!(subtrees.len(), 2);
        assert!(subtrees.iter().any(|s| s.symbol_name == "alpha" && s.kind == "fn"));
        assert!(subtrees.iter().any(|s| s.symbol_name == "Beta" && s.kind == "struct"));
    }
}
