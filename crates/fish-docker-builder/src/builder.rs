use crate::image::DockerImage;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BuildOptions {
    pub tags: Vec<String>,
    pub build_args: Vec<(String, String)>,
    pub no_cache: bool,
    pub target: Option<String>,
}

pub struct DockerBuilder;

impl DockerBuilder {
    pub fn new() -> Self {
        Self
    }

    pub async fn build(
        &self,
        dockerfile_path: &str,
        options: BuildOptions,
    ) -> Result<DockerImage, anyhow::Error> {
        let mut cmd = Command::new("docker");
        cmd.arg("build").arg("-f").arg(dockerfile_path);

        for tag in &options.tags {
            cmd.arg("-t").arg(tag);
        }

        for (k, v) in &options.build_args {
            cmd.arg("--build-arg").arg(format!("{k}={v}"));
        }

        if options.no_cache {
            cmd.arg("--no-cache");
        }

        if let Some(target) = &options.target {
            cmd.arg("--target").arg(target);
        }

        let context_dir = std::path::Path::new(dockerfile_path)
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        cmd.arg(context_dir);

        let output = cmd
            .output()
            .map_err(|e| anyhow::anyhow!("failed to run `docker build`: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "docker build failed with {}: {}",
                output.status,
                stderr.trim()
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let image_id = extract_image_id(&stdout).ok_or_else(|| {
            anyhow::anyhow!(
                "`docker build` succeeded but its output contained no recognizable \
                 image id; refusing to invent one"
            )
        })?;

        Ok(DockerImage {
            id: image_id,
            tags: options.tags,
            // Docker does not report image size on stdout; left at zero
            // rather than estimated.
            size_bytes: 0,
        })
    }
}

/// Pull the real image reference out of build output, handling both modern
/// BuildKit (`writing image sha256:...`) and legacy (`Successfully built
/// <id>`) formats. Returns `None` when neither pattern appears so callers
/// can fail loudly instead of fabricating an identifier.
fn extract_image_id(stdout: &str) -> Option<String> {
    for line in stdout.lines().rev() {
        if let Some(idx) = line.find("sha256:") {
            let hex: String = line[idx + "sha256:".len()..]
                .chars()
                .take_while(|c| c.is_ascii_hexdigit())
                .collect();
            if !hex.is_empty() {
                return Some(format!("sha256:{hex}"));
            }
        }
        if let Some(rest) = line.trim_start().strip_prefix("Successfully built ") {
            let id = rest.trim();
            if !id.is_empty() {
                return Some(id.to_string());
            }
        }
    }
    None
}

impl Default for DockerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extracts_buildkit_image_digest() {
        let stdout = "#12 [4/4] RUN cargo build --release\n\
                      #12 DONE 42.3s\n\
                      #13 exporting to image\n\
                      #13 writing image sha256:ab12cd34ef56\n\
                      #13 naming to docker.io/library/demo:latest done\n";
        assert_eq!(
            extract_image_id(stdout).as_deref(),
            Some("sha256:ab12cd34ef56")
        );
    }

    #[test]
    fn test_extracts_legacy_successfully_built_id() {
        let stdout = "Step 4/4 : RUN cargo build\n Successfully built abc123def\n";
        assert_eq!(extract_image_id(stdout).as_deref(), Some("abc123def"));
    }

    #[test]
    fn test_missing_image_id_is_none_not_fabricated() {
        let stdout = "#5 exporting layers done\n#7 naming done\n";
        assert_eq!(extract_image_id(stdout), None);
    }
}
