#![forbid(unsafe_code)]

//! Automatic cross-language dependency inference.
//!
//! After each detected ecosystem is expanded into its own task subgraph,
//! those subgraphs remain mutually disconnected. This module scans the source
//! files of every discovered project for references that reach into a
//! *sibling* project's directory and turns proven references into real graph
//! edges so producers build first.
//!
//! Two conservative heuristics, both requiring on-disk evidence:
//!
//! - **H1 — relative-path escape**: a quoted string containing `..` that
//!   lexically resolves into another project's directory and exists there
//!   (`import "../../py-worker/contracts/topics.json"`,
//!   `#include "../shared/gen/api.h"`, ...).
//! - **H2 — manifest pointers**: unquoted paths in well-known manifests that
//!   only exist to point across projects (`go.mod` `replace x => ../y`,
//!   `-e ../y` editable requirements).
//!
//! Nothing fuzzy: no package-name guessing, no similarity heuristics. If a
//! reference does not resolve onto disk inside a sibling project, no edge is
//! created. Disable entirely with `--no-infer-deps`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};

use fish_executor::Task;
use fish_graph::{BuildGraph, NodeId};
use fish_incremental::ecosystem::EcosystemType;

/// Directories never descended into while scanning project sources.
const SKIP_DIRS: &[&str] = &[
    ".git",
    ".github",
    ".idea",
    ".venv",
    ".fish",
    "__pycache__",
    "build",
    "deps",
    "dist",
    "fixtures",
    "node_modules",
    "target",
    "vendor",
];

/// File extensions scanned for quoted relative references (H1).
const SCAN_EXTENSIONS: &[&str] = &[
    "c", "cc", "cpp", "cxx", "dart", "go", "h", "hh", "hpp", "java", "js", "json", "jsx", "kt",
    "kts", "mjs", "mts", "py", "rs", "swift", "toml", "ts", "tsx", "yaml", "yml", "zig",
];

/// Manifest files scanned for unquoted cross-project pointers (H2).
const SCAN_MANIFESTS: &[&str] = &["go.mod", "requirements.txt"];

#[derive(Debug, Clone)]
pub struct CrossDepOptions {
    /// Master switch; `false` reproduces the pre-inference behaviour.
    pub enabled: bool,
    /// Safety cap on files scanned per project.
    pub max_files_per_project: usize,
    /// Files larger than this are assumed to be data, not source.
    pub max_file_bytes: u64,
}

impl Default for CrossDepOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            max_files_per_project: 2_000,
            max_file_bytes: 512 * 1024,
        }
    }
}

/// A discovered project root participating in inference.
#[derive(Debug, Clone)]
pub struct ProjectRoot {
    /// Directory containing the project manifest.
    pub dir: PathBuf,
    /// Ecosystem the project was detected as (used only for log labels).
    pub ecosystem: EcosystemType,
}

/// An inferred consumer → producer dependency with its evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InferredEdge {
    /// Directory of the project whose file made the reference.
    pub consumer: PathBuf,
    /// Directory of the project being referenced.
    pub producer: PathBuf,
    /// Human-readable justification shown in build logs.
    pub reason: String,
    /// The file (relative to the workspace root) that proves the reference.
    pub evidence: PathBuf,
}

/// Infer cross-project dependency edges between `projects`.
///
/// Returns one [InferredEdge] per ordered consumer/producer pair (deduplicated
/// across multiple referencing files), sorted deterministically.
pub fn infer_cross_dependencies(
    projects: &[ProjectRoot],
    options: &CrossDepOptions,
) -> Vec<InferredEdge> {
    if !options.enabled || projects.len() < 2 {
        return Vec::new();
    }

    let mut roots: Vec<&ProjectRoot> = projects.iter().collect();
    roots.sort_by(|a, b| a.dir.cmp(&b.dir));
    let root_dirs: Vec<PathBuf> = roots.iter().map(|p| p.dir.clone()).collect();

    let mut edges: BTreeMap<(PathBuf, PathBuf), InferredEdge> = BTreeMap::new();
    for project in &roots {
        let others: Vec<PathBuf> =
            root_dirs.iter().filter(|dir| **dir != project.dir).cloned().collect();

        for (evidence, candidates) in collect_reference_sources(project, &others, options) {
            let mut lines = candidates.lines();
            while let Some(line) = lines.next() {
                for candidate in extract_quoted_candidates(line) {
                    let Some(producer) = resolve_into_sibling(project, &others, &candidate)
                    else {
                        continue;
                    };
                    let key = (project.dir.clone(), producer.clone());
                    edges.entry(key).or_insert_with(|| InferredEdge {
                        consumer: project.dir.clone(),
                        producer: producer.clone(),
                        reason: format!(
                            "{} imports/references `{}` ({})",
                            ecosystem_label(project.ecosystem),
                            candidate,
                            display_relative(&producer)
                        ),
                        evidence: evidence.clone(),
                    });
                }
                for (candidate, kind) in extract_manifest_pointers(line) {
                    let Some(producer) = resolve_into_sibling(project, &others, &candidate)
                    else {
                        continue;
                    };
                    let key = (project.dir.clone(), producer.clone());
                    edges.entry(key).or_insert_with(|| InferredEdge {
                        consumer: project.dir.clone(),
                        producer: producer.clone(),
                        reason: format!(
                            "{} {} points at `{}` ({})",
                            ecosystem_label(project.ecosystem),
                            kind,
                            candidate,
                            display_relative(&producer)
                        ),
                        evidence: evidence.clone(),
                    });
                }
            }
        }
    }

    edges.into_values().collect()
}

/// Apply inferred edges to `graph`, linking every task of the consumer project
/// to every task of the producer project.
///
/// Granularity note: tasks do not reliably declare their outputs today, so v1
/// links whole projects — preferring producer tasks that declare artifacts,
/// and falling back to all of the producer's tasks otherwise. Refining this to
/// per-artifact edges is tracked future work.
///
/// Returns the number of edges added; cycles and duplicates are skipped with
/// a warning instead of failing the build.
pub fn apply_to_graph(
    graph: &mut BuildGraph<Task>,
    node_roots: &HashMap<NodeId, PathBuf>,
    edges: &[InferredEdge],
) -> usize {
    let mut nodes_by_root: HashMap<&PathBuf, Vec<NodeId>> = HashMap::new();
    for (id, root) in node_roots {
        nodes_by_root.entry(root).or_default().push(*id);
    }

    let mut applied = 0;
    for edge in edges {
        let Some(consumer_nodes) = nodes_by_root.get(&edge.consumer) else {
            continue;
        };
        let Some(producer_nodes) = nodes_by_root.get(&edge.producer) else {
            continue;
        };

        // Prefer producers that declare concrete outputs; fall back to all.
        let producing: Vec<NodeId> = producer_nodes
            .iter()
            .copied()
            .filter(|id| {
                graph
                    .node(*id)
                    .is_some_and(|node| !node.payload.artifacts.is_empty())
            })
            .collect();
        let selected = if producing.is_empty() { producer_nodes } else { &producing };

        for &consumer in consumer_nodes {
            for &dependency in selected {
                match graph.add_dependency(dependency, consumer) {
                    Ok(()) => applied += 1,
                    Err(fish_graph::GraphError::Cycle { .. })
                    | Err(fish_graph::GraphError::SelfDependency(_)) => {
                        eprintln!(
                            "warning: cross-project dependency `{}` -> `{}` skipped (cycle)",
                            edge.producer.display(),
                            edge.consumer.display()
                        );
                    }
                    Err(err) => eprintln!("warning: could not link cross-project edge: {err}"),
                }
            }
        }
    }
    applied
}

// ---------------------------------------------------------------------------
// Scanning
// ---------------------------------------------------------------------------

/// Collect `(workspace-relative evidence path, contents)` pairs for every
/// scannable file under `project`, pruning sibling project roots and skipping
/// generated/vendored trees. Returns at most `options.max_files_per_project`
/// entries.
fn collect_reference_sources(
    project: &ProjectRoot,
    siblings: &[PathBuf],
    options: &CrossDepOptions,
) -> Vec<(PathBuf, String)> {
    let sibling_set: HashSet<&PathBuf> = siblings.iter().collect();
    let mut sources = Vec::new();
    let mut stack = vec![project.dir.clone()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if path.is_dir() {
                if SKIP_DIRS.contains(&name_str.as_ref()) || sibling_set.contains(&path) {
                    continue;
                }
                stack.push(path);
            } else if path.is_file() {
                let scannable = SCAN_MANIFESTS.contains(&name_str.as_ref())
                    || path
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .is_some_and(|ext| SCAN_EXTENSIONS.contains(&ext));
                if !scannable || sources.len() >= options.max_files_per_project {
                    continue;
                }
                let Ok(meta) = entry.metadata() else {
                    continue;
                };
                if meta.len() > options.max_file_bytes {
                    continue;
                }
                let Ok(contents) = std::fs::read_to_string(&path) else {
                    continue; // binary or non-UTF-8: no textual references to find
                };
                sources.push((
                    path.strip_prefix(&project.dir).unwrap_or(&path).to_path_buf(),
                    contents,
                ));
            }
        }
    }
    sources
}

/// H1: pull string-literal contents out of a source line and keep those that
/// look like relative paths escaping the current directory.
fn extract_quoted_candidates(line: &str) -> Vec<String> {
    let mut candidates = Vec::new();
    for (index, chunk) in line.split(['\'', '"', '`']).enumerate() {
        if index % 2 == 0 {
            continue; // even chunks are outside string literals
        }
        if chunk.len() > 260 || chunk.contains("://") {
            continue;
        }
        let normalized = chunk.replace('\\', "/");
        if normalized.starts_with("./../")
            || normalized.starts_with("../")
            || (normalized.starts_with("./") && normalized.contains("/../"))
        {
            candidates.push(normalized);
        }
    }
    candidates
}

/// H2: recognize unquoted cross-project pointers in manifest lines.
/// Returns `(path, description)` pairs.
fn extract_manifest_pointers(line: &str) -> Vec<(String, &'static str)> {
    let trimmed = line.trim();

    // go.mod: `replace example.com/x => ../local/x` (target may be quoted)
    if let Some((_, target)) = trimmed.split_once("=>") {
        if let Some(token) = target.split_whitespace().next_back() {
            let token = token.trim_matches('"').replace('\\', "/");
            if token.starts_with("..") {
                return vec![(token, "manifest pointer")];
            }
        }
    }

    // requirements.txt: `-e ../lib` / `--editable ../lib` / plain relative refs
    let token = trimmed
        .strip_prefix("--editable ")
        .or_else(|| trimmed.strip_prefix("-e "))
        .map(str::trim);
    if let Some(token) = token {
        let token = token.trim_matches('"').replace('\\', "/");
        if token.starts_with("..") {
            return vec![(token, "manifest pointer")];
        }
    }

    Vec::new()
}

/// Resolve a reference found in some file of `consumer` and report which
/// sibling project it lands in — but only if the resolved location actually
/// exists on disk.
fn resolve_into_sibling(
    consumer: &ProjectRoot,
    siblings: &[PathBuf],
    reference: &str,
) -> Option<PathBuf> {
    // The referencing file's own directory is unknown at line granularity, so
    // resolve against every directory up to the project root and accept the
    // first hit. This tolerates references from nested src/ directories
    // without parsing each language's module resolution rules.
    for depth in 0..=8 {
        let base = consumer.dir.join("../".repeat(depth));
        let resolved = normalize_lexical(&base.join(reference));
        if !resolved.exists() {
            continue;
        }
        if let Some(producer) = siblings.iter().find(|dir| resolved.starts_with(**dir)) {
            return Some(producer.clone());
        }
    }
    None
}

/// Lexically collapse `.` and `..` components without touching the filesystem.
fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn ecosystem_label(ecosystem: EcosystemType) -> &'static str {
    match ecosystem {
        EcosystemType::Rust => "Rust project",
        EcosystemType::TypeScript => "TypeScript project",
        EcosystemType::Go => "Go project",
        EcosystemType::Python => "Python project",
        EcosystemType::Java => "Java project",
        EcosystemType::DotNet => ".NET project",
        EcosystemType::Cpp => "C/C++ project",
        EcosystemType::Swift => "Swift project",
        EcosystemType::Dart => "Dart project",
        EcosystemType::Zig => "Zig project",
        EcosystemType::Docker => "Docker project",
        EcosystemType::Generic => "Project",
    }
}

/// Best-effort pretty label for log lines: last path segment.
fn display_relative(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_tree(root: &Path, files: &[(&str, &str)]) {
        for (path, contents) in files {
            let full = root.join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(full, contents).unwrap();
        }
    }

    #[test]
    fn h1_detects_ts_import_into_sibling_project() {
        let dir = tempfile::tempdir().unwrap();
        write_tree(
            dir.path(),
            &[
                (
                    "web-frontend/src/index.ts",
                    r#"import { EVENT_TOPICS } from "../../py-worker/contracts/topics.json";
"#,
                ),
                ("web-frontend/package.json", "{ \"name\": \"web\" }"),
                ("py-worker/contracts/topics.json", "{ \"topics\": [] }"),
                ("py-worker/pyproject.toml", "[project]\n"),
            ],
        );

        let projects = vec![
            ProjectRoot { dir: dir.path().join("web-frontend"), ecosystem: EcosystemType::TypeScript },
            ProjectRoot { dir: dir.path().join("py-worker"), ecosystem: EcosystemType::Python },
        ];

        let edges = infer_cross_dependencies(&projects, &CrossDepOptions::default());
        assert_eq!(edges.len(), 1, "edges: {edges:?}");
        assert_eq!(edges[0].consumer.file_name().unwrap(), "web-frontend");
        assert_eq!(edges[0].producer.file_name().unwrap(), "py-worker");
    }

    #[test]
    fn h2_detects_go_mod_replace() {
        let dir = tempfile::tempdir().unwrap();
        write_tree(
            dir.path(),
            &[
                (
                    "api/go.mod",
                    "module demo/api\n\ngo 1.22\n\nrequire demo/contract v0.0.0\n\nreplace demo/contract => ../contract\n",
                ),
                ("api/main.go", "package main\n"),
                ("contract/go.mod", "module demo/contract\n\ngo 1.22\n"),
            ],
        );

        let projects = vec![
            ProjectRoot { dir: dir.path().join("api"), ecosystem: EcosystemType::Go },
            ProjectRoot { dir: dir.path().join("contract"), ecosystem: EcosystemType::Go },
        ];

        let edges = infer_cross_dependencies(&projects, &CrossDepOptions::default());
        assert_eq!(edges.len(), 1, "edges: {edges:?}");
        assert_eq!(edges[0].consumer.file_name().unwrap(), "api");
        assert_eq!(edges[0].producer.file_name().unwrap(), "contract");
    }

    #[test]
    fn missing_target_produces_no_edge() {
        let dir = tempfile::tempdir().unwrap();
        write_tree(
            dir.path(),
            &[
                (
                    "app/src/index.ts",
                    "import { x } from \"../../nowhere/gone.json\";\n",
                ),
                ("app/package.json", "{}"),
                ("other/README.txt", "not an ecosystem"),
            ],
        );

        let projects = vec![
            ProjectRoot { dir: dir.path().join("app"), ecosystem: EcosystemType::TypeScript },
            ProjectRoot { dir: dir.path().join("other"), ecosystem: EcosystemType::Generic },
        ];

        let edges = infer_cross_dependencies(&projects, &CrossDepOptions::default());
        assert!(edges.is_empty(), "unexpected edges: {edges:?}");
    }

    #[test]
    fn urls_are_ignored() {
        let dir = tempfile::tempdir().unwrap();
        write_tree(
            dir.path(),
            &[
                (
                    "app/src/index.ts",
                    "import { x } from \"https://cdn.example.com/lib/../bundle.js\";\n",
                ),
                ("app/package.json", "{}"),
                ("lib/package.json", "{}"),
            ],
        );

        let projects = vec![
            ProjectRoot { dir: dir.path().join("app"), ecosystem: EcosystemType::TypeScript },
            ProjectRoot { dir: dir.path().join("lib"), ecosystem: EcosystemType::TypeScript },
        ];

        let edges = infer_cross_dependencies(&projects, &CrossDepOptions::default());
        assert!(edges.is_empty(), "unexpected edges: {edges:?}");
    }

    #[test]
    fn disabled_option_short_circuits() {
        let dir = tempfile::tempdir().unwrap();
        write_tree(
            dir.path(),
            &[
                ("a/src/index.ts", "import \"../b/x.json\";\n".replace('/', "/").as_str()),
                ("a/package.json", "{}"),
                ("b/x.json", "{}"),
                ("b/package.json", "{}"),
            ],
        );
        let projects = vec![
            ProjectRoot { dir: dir.path().join("a"), ecosystem: EcosystemType::TypeScript },
            ProjectRoot { dir: dir.path().join("b"), ecosystem: EcosystemType::TypeScript },
        ];
        let options = CrossDepOptions { enabled: false, ..CrossDepOptions::default() };
        assert!(infer_cross_dependencies(&projects, &options).is_empty());
    }

    #[test]
    fn apply_links_consumer_after_producer_and_survives_cycles() {
        // Graph layout after merging two single-task subgraphs:
        //   node 0 = producer task, node 1 = consumer task.
        // Adding producer -> consumer must succeed once; the reversed edge
        // would form a cycle and must be skipped with a warning, not panic.
        let mut graph = BuildGraph::new();
        let producer = graph.add_node(Task::new("p-build", "p", Default::default()));
        let consumer = graph.add_node(Task::new("c-build", "c", Default::default()));

        let mut node_roots = HashMap::new();
        node_roots.insert(producer, PathBuf::from("/demo/py-worker"));
        node_roots.insert(consumer, PathBuf::from("/demo/web-frontend"));

        let edge = InferredEdge {
            consumer: PathBuf::from("/demo/web-frontend"),
            producer: PathBuf::from("/demo/py-worker"),
            reason: "test".to_owned(),
            evidence: PathBuf::from("src/index.ts"),
        };

        assert_eq!(apply_to_graph(&mut graph, &node_roots, &[edge.clone()]), 1);
        assert!(graph.deps(consumer).unwrap().contains(&producer));

        let reversed = InferredEdge {
            consumer: PathBuf::from("/demo/py-worker"),
            producer: PathBuf::from("/demo/web-frontend"),
            ..edge
        };
        // producer -> consumer exists, so consumer -> producer is a cycle.
        assert_eq!(apply_to_graph(&mut graph, &node_roots, &[reversed]), 0);
        graph.validate().expect("graph must stay acyclic");
    }
}
