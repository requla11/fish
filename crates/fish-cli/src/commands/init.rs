use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedLanguage {
    pub name: &'static str,
    pub backend: &'static str,
    pub build_cmd: &'static str,
    pub test_cmd: &'static str,
}

pub fn detect_workspace_languages(dir: &Path) -> Vec<DetectedLanguage> {
    let mut detected = Vec::new();
    scan_directory_languages(dir, &mut detected);

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let dir_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if ["packages", "apps", "crates", "services", "modules", "src"].contains(&dir_name)
                {
                    if let Ok(sub_entries) = std::fs::read_dir(&path) {
                        for sub in sub_entries.flatten() {
                            let sub_path = sub.path();
                            if sub_path.is_dir() {
                                scan_directory_languages(&sub_path, &mut detected);
                            }
                        }
                    }
                }
            }
        }
    }

    detected.sort_unstable_by(|a, b| a.backend.cmp(b.backend));
    detected.dedup_by(|a, b| a.backend == b.backend);
    detected
}

fn scan_directory_languages(dir: &Path, detected: &mut Vec<DetectedLanguage>) {
    if dir.join("Cargo.toml").exists() {
        detected.push(DetectedLanguage {
            name: "Rust",
            backend: "rust",
            build_cmd: "cargo build",
            test_cmd: "cargo test",
        });
    }

    if dir.join("package.json").exists() {
        detected.push(DetectedLanguage {
            name: "TypeScript / Node",
            backend: "ts",
            build_cmd: "npm run build",
            test_cmd: "npm test",
        });
    }

    if dir.join("go.mod").exists() {
        detected.push(DetectedLanguage {
            name: "Go",
            backend: "go",
            build_cmd: "go build ./...",
            test_cmd: "go test ./...",
        });
    }

    if dir.join("CMakeLists.txt").exists() || dir.join("Makefile").exists() {
        detected.push(DetectedLanguage {
            name: "C / C++",
            backend: "cc",
            build_cmd: "cmake -B build && cmake --build build",
            test_cmd: "ctest --test-dir build",
        });
    }

    if dir.join("pyproject.toml").exists() || dir.join("requirements.txt").exists() {
        detected.push(DetectedLanguage {
            name: "Python",
            backend: "py",
            build_cmd: "python -m build",
            test_cmd: "pytest",
        });
    }

    if dir.join("pom.xml").exists() || dir.join("build.gradle").exists() {
        detected.push(DetectedLanguage {
            name: "Java",
            backend: "java",
            build_cmd: "mvn compile",
            test_cmd: "mvn test",
        });
    }

    let has_dotnet = if let Ok(entries) = std::fs::read_dir(dir) {
        entries.filter_map(Result::ok).any(|e| {
            let path = e.path();
            if let Some(ext) = path.extension() {
                ext == "csproj" || ext == "sln" || ext == "fsproj"
            } else {
                path.file_name()
                    .map(|n| n == "Directory.Build.props")
                    .unwrap_or(false)
            }
        })
    } else {
        false
    };

    if has_dotnet {
        detected.push(DetectedLanguage {
            name: "Dotnet (.NET)",
            backend: "dotnet",
            build_cmd: "dotnet build",
            test_cmd: "dotnet test",
        });
    }

    if dir.join("Package.swift").exists() {
        detected.push(DetectedLanguage {
            name: "Swift",
            backend: "swift",
            build_cmd: "swift build",
            test_cmd: "swift test",
        });
    }

    if dir.join("pubspec.yaml").exists() {
        detected.push(DetectedLanguage {
            name: "Dart",
            backend: "dart",
            build_cmd: "dart compile exe bin/main.dart",
            test_cmd: "dart test",
        });
    }

    if dir.join("build.zig").exists() {
        detected.push(DetectedLanguage {
            name: "Zig",
            backend: "zig",
            build_cmd: "zig build",
            test_cmd: "zig build test",
        });
    }

    if dir.join("Dockerfile").exists() {
        detected.push(DetectedLanguage {
            name: "Docker",
            backend: "docker",
            build_cmd: "docker build .",
            test_cmd: "docker run --rm test",
        });
    }
}

pub fn generate_fish_yaml(languages: &[DetectedLanguage]) -> String {
    let mut out = String::from(
        "version: \"1\"\n\n# Quantum Polyglot Core (QPC) Engine Enabled\npash:\n  enabled: true\n\ntasks:\n",
    );

    if languages.is_empty() {
        out.push_str("  build:\n    command: echo \"Building project...\"\n\n");
        out.push_str(
            "  test:\n    command: echo \"Running tests...\"\n    depends_on:\n      - build\n",
        );
        return out;
    }

    for lang in languages {
        let prefix = lang.backend;
        out.push_str(&format!("  {prefix}-build:\n"));
        out.push_str(&format!("    command: {}\n", lang.build_cmd));
        out.push_str("    cache:\n      enabled: true\n      morphic: true\n\n");

        out.push_str(&format!("  {prefix}-test:\n"));
        out.push_str(&format!("    command: {}\n", lang.test_cmd));
        out.push_str(&format!("    depends_on:\n      - {prefix}-build\n\n"));
    }

    out.push_str("  build:\n    depends_on:\n");
    for lang in languages {
        out.push_str(&format!("      - {}-build\n", lang.backend));
    }
    out.push_str("\n  test:\n    depends_on:\n");
    for lang in languages {
        out.push_str(&format!("      - {}-test\n", lang.backend));
    }

    out
}

pub fn run_init(path: Option<PathBuf>, force: bool, describe: Option<String>) -> ExitCode {
    let target_dir = path.unwrap_or_else(|| PathBuf::from("."));
    let fish_file = target_dir.join("fish.yaml");

    if fish_file.exists() && !force {
        eprintln!(
            "error: fish.yaml already exists in {}. Use --force to overwrite.",
            target_dir.display()
        );
        return ExitCode::FAILURE;
    }

    println!("🐟 Initializing Fish in {}", target_dir.display());

    if let Some(description) = describe.as_deref().filter(|d| !d.trim().is_empty()) {
        println!("  [info] Description: \"{description}\"");
        let parsed = crate::nl_authoring::parse_description(description);
        if parsed.languages.is_empty() {
            println!(
                "  [warn] No recognized languages in the description; generating default config."
            );
        } else {
            println!(
                "  [ok] Parsed {} language(s) and archetype {:?}:",
                parsed.languages.len(),
                parsed.archetype
            );
            for lang in &parsed.languages {
                println!("    - {} (backend: {})", lang.name, lang.backend);
            }
        }
        let config_content = crate::nl_authoring::generate_from_description(&parsed);
        if let Err(e) = std::fs::write(&fish_file, config_content) {
            eprintln!("error: failed to write fish.yaml: {e}");
            return ExitCode::FAILURE;
        }
        println!("  [ok] Successfully generated {}", fish_file.display());
        println!("\nYou can now run:");
        println!("  fish build    # to build all targets");
        return ExitCode::SUCCESS;
    }

    let detected = detect_workspace_languages(&target_dir);

    if detected.is_empty() {
        println!(
            "  [info] No specific language manifest detected. Generating default configuration."
        );
    } else {
        println!("  [ok] Detected {} language(s):", detected.len());
        for lang in &detected {
            println!("    - {} (backend: {})", lang.name, lang.backend);
        }
    }

    let config_content = generate_fish_yaml(&detected);
    if let Err(e) = std::fs::write(&fish_file, config_content) {
        eprintln!("error: failed to write fish.yaml: {e}");
        return ExitCode::FAILURE;
    }

    println!("  [ok] Successfully generated {}", fish_file.display());
    println!("\nYou can now run:");
    println!("  fish build    # to build all targets");
    println!("  fish test     # to execute test suites");
    println!("  fish graph    # to visualize task dependency graph");

    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_workspace_languages() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path();

        std::fs::write(path.join("Cargo.toml"), "").unwrap();
        std::fs::write(path.join("package.json"), "").unwrap();
        std::fs::write(path.join("App.csproj"), "").unwrap();

        let detected = detect_workspace_languages(path);
        let names: Vec<&str> = detected.iter().map(|d| d.name).collect();

        assert!(names.contains(&"Rust"));
        assert!(names.contains(&"TypeScript / Node"));
        assert!(names.contains(&"Dotnet (.NET)"));

        let yaml_content = generate_fish_yaml(&detected);
        assert!(yaml_content.contains("rust-build:"));
        assert!(yaml_content.contains("ts-build:"));
        assert!(yaml_content.contains("dotnet-build:"));
    }

    #[test]
    fn test_nested_monorepo_discovery() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path();

        let apps_dir = path.join("apps").join("web");
        let packages_dir = path.join("packages").join("core");
        std::fs::create_dir_all(&apps_dir).unwrap();
        std::fs::create_dir_all(&packages_dir).unwrap();

        std::fs::write(apps_dir.join("package.json"), "").unwrap();
        std::fs::write(packages_dir.join("go.mod"), "").unwrap();

        let detected = detect_workspace_languages(path);
        let names: Vec<&str> = detected.iter().map(|d| d.name).collect();

        assert!(names.contains(&"TypeScript / Node"));
        assert!(names.contains(&"Go"));
    }
}
