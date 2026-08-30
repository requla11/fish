#![forbid(unsafe_code)]

pub mod banana_oci;
pub mod config;
pub mod ecosystem;
pub mod fingerprint;
pub mod toolchain;

pub use banana_oci::FishOciCompiler;
pub use config::DockerProjectConfig;
pub use fingerprint::DockerFingerprinter;
pub use toolchain::DockerToolchain;

use fish_core::BuildBackend;
use fish_executor::CommandSpec;
use fish_graph::BuildGraph;
use std::fmt::Debug;

type DockerStages = Vec<(String, Vec<CommandSpec>)>;

/// Derive a stable task-name segment from a base-image reference when a FROM
/// line carries no AS alias ("ubuntu:22.04" -> "ubuntu-22.04").
fn sanitize_image(image: &str) -> String {
    let mut name: String = image
        .rsplit('/')
        .next()
        .unwrap_or(image)
        .chars()
        .map(|c| if c == ':' || c == '.' { '-' } else { c })
        .collect();
    if name.is_empty() {
        name = "default".to_string();
    }
    name
}

#[derive(Debug)]
pub struct DockerBackend {
    config: DockerProjectConfig,
    toolchain: DockerToolchain,
    fingerprinter: DockerFingerprinter,
}

impl DockerBackend {
    pub fn new(config: DockerProjectConfig) -> Result<Self, anyhow::Error> {
        let toolchain = DockerToolchain::detect()?;
        let fingerprinter = DockerFingerprinter::new(config.clone());

        Ok(Self {
            config,
            toolchain,
            fingerprinter,
        })
    }

    pub fn with_toolchain(config: DockerProjectConfig, toolchain: DockerToolchain) -> Self {
        let fingerprinter = DockerFingerprinter::new(config.clone());
        Self {
            config,
            toolchain,
            fingerprinter,
        }
    }

    pub fn detect_config(project_dir: &std::path::Path) -> Option<DockerProjectConfig> {
        let dockerfile = project_dir.join("Dockerfile");
        if dockerfile.exists() {
            return Some(DockerProjectConfig {
                dockerfile_path: Some(dockerfile),
                context_path: project_dir.to_path_buf(),
                build_args: std::collections::HashMap::new(),
                target: None,
                cache_from: Vec::new(),
                cache_to: Vec::new(),
            });
        }

        let compose_file = project_dir.join("docker-compose.yml");
        if compose_file.exists() {
            return Some(DockerProjectConfig {
                dockerfile_path: None,
                context_path: project_dir.to_path_buf(),
                build_args: std::collections::HashMap::new(),
                target: None,
                cache_from: Vec::new(),
                cache_to: Vec::new(),
            });
        }

        None
    }

    pub fn build_task_graph(&self) -> Result<BuildGraph<fish_executor::Task>, anyhow::Error> {
        let mut graph = BuildGraph::new();
        let fingerprint = self.fingerprinter.compute()?;
        let stages = self.parse_dockerfile()?;
        let stages_count = stages.len();

        for (index, (stage_name, commands)) in stages.into_iter().enumerate() {
            let task_id = format!("docker:build:{}", stage_name);
            let is_last = index + 1 == stages_count;

            // One task per stage. Every parsed instruction carries the same
            // whole-stage `docker build` invocation, so emitting one node per
            // instruction would schedule N redundant concurrent builds under
            // a single task name.
            let Some(cmd) = commands.into_iter().next() else {
                continue;
            };

            let mut task = fish_executor::Task::new(
                task_id,
                format!("Build Docker stage: {}", stage_name),
                cmd,
            );

            if let Some(dockerfile) = &self.config.dockerfile_path {
                task = task.with_cache(fish_executor::CacheEntry {
                    key: format!("docker:{}", dockerfile.display()),
                    fingerprint: fingerprint.clone(),
                });
            }

            if is_last {
                task = task.with_artifacts(vec![std::path::PathBuf::from(format!(
                    "{}.tar",
                    stage_name
                ))]);
            }

            let _node_id = graph.add_node(task);
        }

        Ok(graph)
    }

    pub fn validate_config(&self) -> Result<(), anyhow::Error> {
        if let Some(dockerfile) = &self.config.dockerfile_path
            && !dockerfile.exists()
        {
            return Err(anyhow::anyhow!(
                "Dockerfile not found: {}",
                dockerfile.display()
            ));
        }
        Ok(())
    }
}

impl BuildBackend for DockerBackend {
    fn name(&self) -> &'static str {
        "docker"
    }
}

impl DockerBackend {
    fn parse_dockerfile(&self) -> Result<DockerStages, anyhow::Error> {
        let dockerfile = self
            .config
            .dockerfile_path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No Dockerfile specified"))?;

        let content = std::fs::read_to_string(dockerfile)?;
        let mut stages = Vec::new();
        // Stages begin at each FROM. The very first FROM is the implicit
        // "default" stage; later ones are named by their AS alias, or by a
        // sanitized image reference when no alias is given.
        let mut current_stage = "default".to_string();
        let mut current_commands = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Some(rest) = line.strip_prefix("FROM ") {
                if !current_commands.is_empty() {
                    stages.push((current_stage.clone(), std::mem::take(&mut current_commands)));
                }
                let tokens: Vec<&str> = rest.split_whitespace().collect();
                current_stage = match tokens.as_slice() {
                    [_, alias_word, alias] if alias_word.eq_ignore_ascii_case("as") => {
                        (*alias).to_string()
                    }
                    [image, ..] => sanitize_image(image),
                    [] => "default".to_string(),
                };
            } else if line.starts_with("RUN ")
                || line.starts_with("COPY ")
                || line.starts_with("ADD ")
            {
                let docker_cmd = format!(
                    "build -t stage:{} {}",
                    current_stage,
                    self.config.context_path.display()
                );
                let mut cmd =
                    CommandSpec::new(self.toolchain.docker_path.to_string_lossy().to_string());
                for arg in docker_cmd.split_whitespace() {
                    cmd = cmd.arg(arg);
                }
                current_commands.push(cmd);
            }
        }

        if !current_commands.is_empty() {
            stages.push((current_stage, current_commands));
        }

        if stages.is_empty() {
            let docker_cmd = format!(
                "build -t fish:latest {}",
                self.config.context_path.display()
            );
            let mut cmd =
                CommandSpec::new(self.toolchain.docker_path.to_string_lossy().to_string());
            for arg in docker_cmd.split_whitespace() {
                cmd = cmd.arg(arg);
            }
            stages.push(("default".to_string(), vec![cmd]));
        }

        Ok(stages)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_backend_creation() {
        let config = DockerProjectConfig {
            dockerfile_path: Some(std::path::PathBuf::from("Dockerfile")),
            context_path: std::path::PathBuf::from("."),
            build_args: std::collections::HashMap::new(),
            target: None,
            cache_from: Vec::new(),
            cache_to: Vec::new(),
        };

        let toolchain = DockerToolchain {
            docker_path: std::path::PathBuf::from("docker"),
            is_podman: false,
            version: "24.0.0".to_string(),
        };

        let backend = DockerBackend::with_toolchain(config, toolchain);
        assert_eq!(backend.config.context_path, std::path::PathBuf::from("."));
    }

    fn write_dockerfile(dir: &std::path::Path, content: &str) -> DockerProjectConfig {
        let path = dir.join("Dockerfile");
        std::fs::write(&path, content).unwrap();
        DockerProjectConfig {
            dockerfile_path: Some(path),
            context_path: dir.to_path_buf(),
            build_args: std::collections::HashMap::new(),
            target: None,
            cache_from: Vec::new(),
            cache_to: Vec::new(),
        }
    }

    fn fake_toolchain() -> DockerToolchain {
        DockerToolchain {
            docker_path: std::path::PathBuf::from("docker"),
            is_podman: false,
            version: "24.0.0".to_string(),
        }
    }

    #[test]
    fn lowercase_as_aliases_are_recognized() {
        // Regression: the stage splitter matched " AS " case-sensitively, so
        // the conventional lowercase `as` silently collapsed every stage into
        // one "default" bucket.
        let dir = tempfile::tempdir().unwrap();
        let config = write_dockerfile(
            dir.path(),
            "FROM rust:1.70 as rust-builder\nRUN cargo build\n\nFROM golang:1.21 AS go-builder\nRUN go build\n",
        );
        let backend = DockerBackend::with_toolchain(config, fake_toolchain());
        let stages = backend.parse_dockerfile().unwrap();

        let names: Vec<&str> = stages.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["rust-builder", "go-builder"]);
    }

    #[test]
    fn unnamed_from_stages_fall_back_to_sanitized_image_names() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_dockerfile(
            dir.path(),
            "FROM alpine:3.19\nRUN apk add curl\n\nFROM ubuntu:22.04\nRUN apt update\n",
        );
        let backend = DockerBackend::with_toolchain(config, fake_toolchain());
        let stages = backend.parse_dockerfile().unwrap();

        let names: Vec<&str> = stages.iter().map(|(name, _)| name.as_str()).collect();
        assert_eq!(names, ["alpine-3-19", "ubuntu-22-04"]);
    }

    #[test]
    fn one_task_per_stage_not_per_instruction() {
        // Regression: nine instructions under one stage produced nine
        // identically-named tasks â€” nine concurrent full-image builds.
        let dir = tempfile::tempdir().unwrap();
        let config = write_dockerfile(
            dir.path(),
            "FROM alpine:3.19\nRUN a\nCOPY x .\nADD y .\nRUN b\nRUN c\nRUN d\nRUN e\nRUN f\nRUN g\n",
        );
        let backend = DockerBackend::with_toolchain(config, fake_toolchain());
        let graph = backend.build_task_graph().unwrap();

        assert_eq!(graph.len(), 1, "instructions must collapse into one task");
        let node = &graph.nodes()[0];
        // A lone FROM names its stage after the sanitized image reference.
        assert_eq!(node.payload.label, "docker:build:alpine-3-19");
        assert!(
            !node.payload.artifacts.is_empty(),
            "single-stage build IS the final stage"
        );
    }

    #[test]
    fn multi_stage_yields_one_node_per_stage() {
        let dir = tempfile::tempdir().unwrap();
        let config = write_dockerfile(
            dir.path(),
            "FROM rust:1.70 as builder\nRUN cargo build\n\nFROM alpine:3.19\nRUN echo hi\n",
        );
        let backend = DockerBackend::with_toolchain(config, fake_toolchain());
        let graph = backend.build_task_graph().unwrap();

        assert_eq!(graph.len(), 2);
        let labels: Vec<&str> = graph
            .nodes()
            .iter()
            .map(|n| n.payload.label.as_str())
            .collect();
        assert!(labels.contains(&"docker:build:builder"));
        // The final stage declares the image artifact.
        let final_node = graph
            .nodes()
            .iter()
            .find(|n| n.payload.label == "docker:build:alpine-3-19")
            .expect("final stage task");
        assert!(!final_node.payload.artifacts.is_empty());
    }
}
