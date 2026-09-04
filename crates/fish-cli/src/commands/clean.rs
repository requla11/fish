use crate::utils::resolve_start_dir;
use fish_core::project::Project;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub fn run_clean(path: Option<&Path>, all: bool) -> ExitCode {
    let start_dir = match resolve_start_dir(path) {
        Ok(dir) => dir,
        Err(message) => {
            eprintln!("error: {message}");
            return ExitCode::FAILURE;
        }
    };

    let project = match Project::discover(&start_dir) {
        Ok(Some(project)) => project,
        Ok(None) => {
            eprintln!(
                "error: no Cargo project found in `{}` or any parent directory",
                start_dir.display()
            );
            return ExitCode::FAILURE;
        }
        Err(error) => {
            eprintln!("error: {error}");
            eprintln!("hint: make sure `cargo` is installed and available on PATH");
            return ExitCode::FAILURE;
        }
    };

    let workspace_root = project.workspace_root().to_path_buf();
    println!(
        "Cleaning: {}",
        project.workspace_root().as_std_path().display()
    );
    match std::process::Command::new("cargo")
        .arg("clean")
        .current_dir(workspace_root)
        .status()
    {
        Ok(status) if status.success() => {
            println!("Cleaned.");
            if all {
                let cache_dir = std::env::var("FISH_CACHE_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| {
                        if let Ok(home) =
                            std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
                        {
                            PathBuf::from(home).join(".fish").join("cache")
                        } else {
                            std::env::temp_dir().join("fish").join("cache")
                        }
                    });
                if cache_dir.exists() {
                    let _ = std::fs::remove_dir_all(&cache_dir);
                    println!("Cleared cache directory at: {}", cache_dir.display());
                }
            }
            ExitCode::SUCCESS
        }
        Ok(status) => {
            eprintln!("error: `cargo clean` failed with {status}");
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("error: failed to run `cargo clean`: {error}");
            ExitCode::FAILURE
        }
    }
}
