#![forbid(unsafe_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::backend_registry;
use crate::cross_deps::{CrossDepOptions, ProjectRoot};
use fish_backend_api::Ecosystem;
use fish_backend_rust::{BuildMode, RustBackend};
use fish_core::project::Project;
use fish_executor::Task;
use fish_graph::{BuildGraph, NodeId};
use fish_incremental::ecosystem::{EcosystemInfo, EcosystemType};

/// Map the incremental scanner's ecosystem tag onto the backend API's.
fn ecosystem_of(t: EcosystemType) -> Option<Ecosystem> {
    match t {
        EcosystemType::Rust => Some(Ecosystem::Rust),
        EcosystemType::TypeScript => Some(Ecosystem::TypeScript),
        EcosystemType::Go => Some(Ecosystem::Go),
        EcosystemType::Python => Some(Ecosystem::Python),
        EcosystemType::Cpp => Some(Ecosystem::Cpp),
        EcosystemType::Java => Some(Ecosystem::Java),
        EcosystemType::DotNet => Some(Ecosystem::DotNet),
        EcosystemType::Swift => Some(Ecosystem::Swift),
        EcosystemType::Dart => Some(Ecosystem::Dart),
        EcosystemType::Zig => Some(Ecosystem::Zig),
        EcosystemType::Docker => Some(Ecosystem::Docker),
        _ => None,
    }
}

/// Short label for log lines: the project directory's file name.
fn project_label(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

pub struct PolyglotGraphBuilder;

impl PolyglotGraphBuilder {
    /// Expand every ecosystem in `ecosystems` into tasks and merge them into
    /// one DAG. Cross-language dependency inference runs unless disabled
    /// through `options`. Callers that have not scanned yet should run
    /// `detect_ecosystems(start_dir)` themselves and pass the result here —
    /// dispatch sites typically need the scan anyway, so this keeps the whole
    /// command to a single full-tree walk.
    pub fn build_unified_graph_from_ecosystems(
        start_dir: &Path,
        ecosystems: Vec<EcosystemInfo>,
        mode: BuildMode,
        options: &CrossDepOptions,
    ) -> Result<BuildGraph<Task>, anyhow::Error> {
        if ecosystems.is_empty() {
            if let Ok(Some(project)) = Project::discover(start_dir) {
                let backend = RustBackend::new()?;
                let package_graph = if mode == BuildMode::Test {
                    project.build_test_graph()?
                } else {
                    project.build_graph()?
                };
                return backend
                    .create_tasks(&project, &package_graph, mode, true)
                    .map_err(Into::into);
            }
            return Err(anyhow::anyhow!(
                "No supported build ecosystem detected in `{}`",
                start_dir.display()
            ));
        }

        let mut master_graph = BuildGraph::new();
        // Per-project task lists; keyed by the directory whose manifest
        // produced the subgraph so inferred edges can map projects onto nodes.
        let mut detected_projects: Vec<ProjectRoot> = Vec::new();
        let mut nodes_by_project: HashMap<PathBuf, Vec<NodeId>> = HashMap::new();
        // Docker tasks gathered up front: every non-Docker project must be
        // linked against ALL Docker projects, never an arbitrary one.
        let mut docker_task_nodes: Vec<NodeId> = Vec::new();
        let mut processed_rust = false;

        for info in &ecosystems {
            if info.ecosystem == EcosystemType::Rust {
                if processed_rust {
                    continue;
                }
                processed_rust = true;
            }
            let manifest_dir =
                if info.ecosystem == EcosystemType::Rust && start_dir.join("Cargo.toml").exists() {
                    start_dir
                } else {
                    info.manifest_path.parent().unwrap_or(start_dir)
                };
            let sub_graph_res = Self::build_subgraph_for_ecosystem(info, manifest_dir, mode);
            if let Ok(sub_graph) = sub_graph_res
                && !sub_graph.is_empty()
            {
                let id_map = master_graph.merge_subgraph(sub_graph);
                let new_ids: Vec<NodeId> = id_map.into_values().collect();
                if info.ecosystem == EcosystemType::Docker {
                    docker_task_nodes.extend(new_ids.iter().copied());
                }
                // One directory can host several ecosystems (a Dockerfile
                // beside a go.mod, say); their tasks share a single bucket so
                // neither subgraph ends up orphaned from inference and the
                // Docker pass.
                match nodes_by_project.entry(manifest_dir.to_path_buf()) {
                    std::collections::hash_map::Entry::Occupied(mut occupied) => {
                        occupied.get_mut().extend(new_ids);
                    }
                    std::collections::hash_map::Entry::Vacant(vacant) => {
                        vacant.insert(new_ids);
                        detected_projects.push(ProjectRoot {
                            dir: manifest_dir.to_path_buf(),
                            ecosystem: info.ecosystem,
                        });
                    }
                }
            }
        }

        // Cross-language inference runs BEFORE the Docker pass so that Docker
        // tasks end up downstream of inferred producers as well.
        if options.enabled && detected_projects.len() > 1 {
            let inferable: Vec<ProjectRoot> = detected_projects
                .iter()
                .filter(|project| project.ecosystem != EcosystemType::Docker)
                .cloned()
                .collect();
            let edges = crate::cross_deps::infer_cross_dependencies(&inferable, options);
            if !edges.is_empty() {
                println!("🔗 Inferring cross-language dependencies:");
                for edge in &edges {
                    println!(
                        "   ↳ {} → {} ({})",
                        project_label(&edge.consumer),
                        project_label(&edge.producer),
                        edge.reason
                    );
                }
                let applied =
                    crate::cross_deps::apply_to_graph(&mut master_graph, &nodes_by_project, &edges);
                println!(
                    "🔗 Linked {applied} cross-project task edge(s) from {} inference(s) (disable with --no-infer-deps)",
                    edges.len()
                );
            }
        }

        if !docker_task_nodes.is_empty() {
            let docker_set: HashSet<NodeId> = docker_task_nodes.iter().copied().collect();
            for nodes in nodes_by_project.values() {
                for &dep in nodes {
                    // Docker images never order against each other.
                    if docker_set.contains(&dep) {
                        continue;
                    }
                    for &docker_node in &docker_task_nodes {
                        let _ = master_graph.add_dependency(dep, docker_node);
                    }
                }
            }
        }

        if master_graph.is_empty()
            && let Ok(Some(project)) = Project::discover(start_dir)
        {
            let backend = RustBackend::new()?;
            let package_graph = if mode == BuildMode::Test {
                project.build_test_graph()?
            } else {
                project.build_graph()?
            };
            return backend
                .create_tasks(&project, &package_graph, mode, true)
                .map_err(Into::into);
        }

        Ok(master_graph)
    }

    fn build_subgraph_for_ecosystem(
        info: &EcosystemInfo,
        dir: &Path,
        mode: BuildMode,
    ) -> Result<BuildGraph<Task>, anyhow::Error> {
        // Dispatch is data-driven: each backend owns its config
        // discovery and graph construction behind the uniform
        // EcosystemBackend contract.
        match ecosystem_of(info.ecosystem) {
            Some(eco) => {
                backend_registry::build_subgraph(eco, dir, mode).map_err(anyhow::Error::msg)
            }
            None => Ok(BuildGraph::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fish_incremental::ecosystem::detect_ecosystems;
    use tempfile::tempdir;

    #[test]
    fn test_polyglot_graph_builder_empty_dir() {
        let dir = tempdir().unwrap();
        let res = PolyglotGraphBuilder::build_unified_graph_from_ecosystems(
            dir.path(),
            detect_ecosystems(dir.path()),
            BuildMode::Build,
            &CrossDepOptions::default(),
        );
        assert!(res.is_err());
    }

    #[test]
    fn test_polyglot_graph_builder_rust_project() {
        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        std::fs::write(
            &cargo_toml,
            r#"[package]
name = "test_rust_pkg"
version = "0.1.0"
edition = "2021"
"#,
        )
        .unwrap();
        let src_dir = dir.path().join("src");
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::write(src_dir.join("lib.rs"), "").unwrap();

        let graph = PolyglotGraphBuilder::build_unified_graph_from_ecosystems(
            dir.path(),
            detect_ecosystems(dir.path()),
            BuildMode::Build,
            &CrossDepOptions::default(),
        )
        .unwrap();
        assert_eq!(graph.len(), 1);
    }

    #[test]
    fn unified_graph_infers_cross_language_edges() {
        // A Python project owns a JSON contract; a TypeScript project imports
        // it across the directory boundary. Inference must wire the two
        // subgraphs together; disabling inference must leave them isolated.
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("py-worker/contracts")).unwrap();
        // Explicit task configs keep the fixture deterministic: the
        // default py/ts task sets gate on host tooling (ruff, tsc, uv)
        // which varies between developer machines and CI runners.
        std::fs::write(
            dir.path().join("py-worker/pyproject.toml"),
            "[project]
name = \"pyw\"
version = \"0.1.0\"
",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("py-worker/fish.py.json"),
            "{\"name\": \"pyw\", \"tasks\": [{\"name\": \"validate\", \"command\": \"python\", \"args\": [\"-c\", \"pass\"], \"depends_on\": []}]}",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("py-worker/contracts/topics.json"),
            "{\"topics\": []}",
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("web-frontend/src")).unwrap();
        std::fs::write(
            dir.path().join("web-frontend/package.json"),
            "{\"name\": \"web\"}",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("web-frontend/fish.ts.json"),
            "{\"name\": \"web\", \"tasks\": [{\"name\": \"build\", \"command\": \"node\", \"args\": [\"-e\", \"0\"], \"depends_on\": []}]}",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("web-frontend/src/index.ts"),
            "import { topics } from \"../../py-worker/contracts/topics.json\";\nconsole.log(topics);\n",
        )
        .unwrap();

        let total_deps = |graph: &BuildGraph<Task>| {
            graph
                .nodes()
                .iter()
                .map(|node| graph.deps(node.id).map(|deps| deps.len()).unwrap_or(0))
                .sum::<usize>()
        };

        let linked = PolyglotGraphBuilder::build_unified_graph_from_ecosystems(
            dir.path(),
            detect_ecosystems(dir.path()),
            BuildMode::Build,
            &CrossDepOptions::default(),
        )
        .unwrap();
        let isolated = PolyglotGraphBuilder::build_unified_graph_from_ecosystems(
            dir.path(),
            detect_ecosystems(dir.path()),
            BuildMode::Build,
            &CrossDepOptions {
                enabled: false,
                ..CrossDepOptions::default()
            },
        )
        .unwrap();

        assert_eq!(linked.len(), isolated.len(), "same tasks either way");
        assert!(
            total_deps(&linked) > total_deps(&isolated),
            "inference must add edges (linked: {}, isolated: {})",
            total_deps(&linked),
            total_deps(&isolated)
        );
        linked.validate().expect("linked graph must stay acyclic");
    }
}
