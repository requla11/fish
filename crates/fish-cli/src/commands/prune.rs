use std::process::ExitCode;

use fish_core::project::Project;
use fish_core::prune::prune_workspace;

use crate::args::PruneArgs;
use crate::utils;

pub fn run_prune(args: PruneArgs) -> ExitCode {
    let start_dir = match utils::resolve_start_dir(args.path.as_deref()) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let project = match Project::discover(&start_dir) {
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

    let out_dir = if args.out_dir.is_relative() {
        start_dir.join(&args.out_dir)
    } else {
        args.out_dir
    };

    match prune_workspace(&project, &args.scope, &out_dir, args.docker) {
        Ok(result) => {
            println!(
                "Successfully pruned monorepo for target '{}' into {}",
                result.target,
                result.out_dir.display()
            );
            println!(
                "Included {} packages: {}",
                result.packages_included.len(),
                result.packages_included.join(", ")
            );
            println!(
                "Copied {} files ({:.2} MB). Output layout: 'json/' and 'full/' ready for Docker caching.",
                result.files_copied,
                (result.bytes_copied as f64) / (1024.0 * 1024.0)
            );
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("error: pruning failed: {err}");
            ExitCode::FAILURE
        }
    }
}
