use std::path::Path;
use std::process::ExitCode;

use fish_core::boundary::{BoundaryEnforcer, PackageBoundaryMeta};

use crate::args::BoundariesArgs;
use crate::config::FishConfig;
use crate::utils;

pub fn check_boundaries(start_dir: &Path, config: &FishConfig, json: bool) -> ExitCode {
    if config.boundaries.is_empty() {
        if !json {
            println!("No architectural boundary rules configured in fish.toml.");
        }
        return ExitCode::SUCCESS;
    }

    let enforcer = BoundaryEnforcer::new(config.boundaries.clone());

    let project = match fish_core::project::Project::discover(start_dir) {
        Ok(Some(proj)) => proj,
        Ok(None) => {
            eprintln!(
                "error: no fish project discovered at {}",
                start_dir.display()
            );
            return ExitCode::FAILURE;
        }
        Err(err) => {
            eprintln!("error: failed to discover project: {err}");
            return ExitCode::FAILURE;
        }
    };

    let mut packages = Vec::new();
    for pkg in project
        .packages_for_paths(&[])
        .map(|_| Vec::new())
        .unwrap_or_else(|| {
            project
                .metadata()
                .workspace_members
                .iter()
                .filter_map(|id| project.package(id))
                .cloned()
                .collect()
        })
    {
        let mut tags = Vec::new();
        let path_str = pkg.manifest_path.to_string();
        if path_str.contains("apps") || path_str.contains("cli") {
            tags.push("app".to_string());
        } else if path_str.contains("packages") || path_str.contains("crates") {
            tags.push("lib".to_string());
        }
        if path_str.contains("backend") {
            tags.push("backend".to_string());
        }
        if path_str.contains("ui") || path_str.contains("frontend") {
            tags.push("frontend".to_string());
        }

        let deps: Vec<String> = pkg.dependencies.iter().map(|d| d.name.clone()).collect();

        packages.push(PackageBoundaryMeta {
            name: pkg.name.to_string(),
            tags,
            dependencies: deps,
        });
    }

    match enforcer.check(&packages) {
        Ok(()) => {
            if json {
                println!(r#"{{"status":"ok","violations":[]}}"#);
            } else {
                println!(
                    "Boundary verification passed: {} packages checked against {} rules.",
                    packages.len(),
                    config.boundaries.len()
                );
            }
            ExitCode::SUCCESS
        }
        Err(violations) => {
            if json {
                let out = serde_json::json!({
                    "status": "violation",
                    "violations": violations
                });
                println!("{out}");
            } else {
                eprintln!(
                    "Architectural boundary violations detected ({}):",
                    violations.len()
                );
                for v in violations {
                    eprintln!("  - {v}");
                }
            }
            ExitCode::FAILURE
        }
    }
}

pub fn run_boundaries(args: BoundariesArgs) -> ExitCode {
    let start_dir = match utils::resolve_start_dir(args.path.as_deref()) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let config = match FishConfig::load(&start_dir) {
        Ok(Some(config)) => config,
        Ok(None) => FishConfig::default(),
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    check_boundaries(&start_dir, &config, args.json)
}
