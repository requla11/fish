use std::collections::HashSet;
use std::process::ExitCode;

use fish_backend_rust::BuildMode;
use fish_core::project::Project;
use fish_graph::BuildGraph;

use crate::args::{GraphArgs, GraphFormat};
use crate::cross_deps::CrossDepOptions;
use crate::render;
use crate::utils::resolve_start_dir;

pub fn run_graph(args: GraphArgs) -> ExitCode {
    let start_dir = match resolve_start_dir(args.path.as_deref()) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    // Polyglot workspaces (more than one detected ecosystem) are rendered
    // from the unified task graph — the same graph `fish build` executes —
    // so cross-language edges appear here exactly as they are scheduled.
    let ecosystems = fish_incremental::ecosystem::detect_ecosystems(&start_dir);
    let unique_ecosystems: HashSet<_> = ecosystems.iter().map(|e| e.ecosystem).collect();
    if unique_ecosystems.len() > 1 {
        let cross_dep_options = CrossDepOptions {
            enabled: !args.no_infer_deps,
            ..CrossDepOptions::default()
        };
        match crate::polyglot::PolyglotGraphBuilder::build_unified_graph_from_ecosystems(
            &start_dir,
            ecosystems,
            BuildMode::Build,
            &cross_dep_options,
        ) {
            Ok(graph) if !graph.is_empty() => {
                return render_polyglot_graph(&graph, &args.format);
            }
            Ok(_) => {} // no polyglot tasks materialized; fall back to Cargo
            Err(error) => {
                eprintln!("error: polyglot graph resolution failed: {error}");
                return ExitCode::FAILURE;
            }
        }
    }

    let project = match Project::discover(&start_dir) {
        Ok(Some(project)) => project,
        Ok(None) => {
            eprintln!(
                "error: no buildable project found in `{}` or any parent directory",
                start_dir.display()
            );
            return ExitCode::FAILURE;
        }
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };

    let graph = match project.build_graph() {
        Ok(graph) => graph,
        Err(error) => {
            eprintln!("error: {error}");
            return ExitCode::FAILURE;
        }
    };

    match args.format {
        GraphFormat::Tree => render::print_graph_tree(&project, &graph),
        GraphFormat::Json => render::print_graph_json(&project, &graph),
        GraphFormat::Dot => render::print_graph_dot(&project, &graph),
    }

    ExitCode::SUCCESS
}

fn render_polyglot_graph(
    graph: &BuildGraph<fish_executor::Task>,
    format: &GraphFormat,
) -> ExitCode {
    match format {
        GraphFormat::Tree => render::print_task_graph_tree(graph),
        GraphFormat::Json => render::print_task_graph_json(graph),
        GraphFormat::Dot => render::print_task_graph_dot(graph),
    }
    ExitCode::SUCCESS
}
