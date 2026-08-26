#![forbid(unsafe_code)]

use crate::ZigBackendError;

#[derive(Debug, Clone)]
pub struct ZigToolchain {
    pub executable: String,
    pub zig_version: String,
}

#[derive(Debug, Clone)]
pub enum ZigCompiler {
    Zig(String),
}

impl ZigToolchain {
    /// Cached process-wide: PATH probe + `zig version` per call
    /// previously ran once per project directory.
    pub fn detect() -> Result<Self, ZigBackendError> {
        static CACHE: std::sync::OnceLock<Result<ZigToolchain, String>> =
            std::sync::OnceLock::new();
        CACHE
            .get_or_init(|| Self::detect_uncached().map_err(|e| e.to_string()))
            .clone()
            .map_err(ZigBackendError::Toolchain)
    }

    fn detect_uncached() -> Result<Self, ZigBackendError> {
        let zig_executable = Self::find_executable("zig")
            .ok_or_else(|| ZigBackendError::Toolchain("Zig not found in PATH".to_string()))?;

        let zig_version = Self::get_version(&zig_executable, &["version"])
            .unwrap_or_else(|_| "unknown".to_string());

        Ok(ZigToolchain {
            executable: zig_executable,
            zig_version,
        })
    }

    pub fn with_zig(executable: String) -> Self {
        let zig_version =
            Self::get_version(&executable, &["version"]).unwrap_or_else(|_| "unknown".to_string());

        ZigToolchain {
            executable: executable.clone(),
            zig_version,
        }
    }

    fn find_executable(name: &str) -> Option<String> {
        if let Ok(output) = std::process::Command::new("where").arg(name).output()
            && output.status.success()
        {
            let paths = String::from_utf8_lossy(&output.stdout);
            return paths.lines().next().map(|s| s.to_string());
        }

        if std::process::Command::new(name)
            .arg("version")
            .output()
            .is_ok()
        {
            return Some(name.to_string());
        }

        None
    }

    fn get_version(executable: &str, args: &[&str]) -> Result<String, ZigBackendError> {
        let output = std::process::Command::new(executable)
            .args(args)
            .output()
            .map_err(|e| {
                ZigBackendError::Toolchain(format!("Failed to run {}: {}", executable, e))
            })?;

        if !output.status.success() {
            return Err(ZigBackendError::Toolchain(format!(
                "{} exited with error code: {:?}",
                executable,
                output.status.code()
            )));
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.lines().next().unwrap_or("unknown").to_string())
    }
}

impl ZigCompiler {
    pub fn detect() -> Result<Self, ZigBackendError> {
        if let Some(zig) = ZigToolchain::find_executable("zig") {
            return Ok(ZigCompiler::Zig(zig));
        }

        Err(ZigBackendError::Toolchain(
            "Zig compiler not found".to_string(),
        ))
    }

    pub fn executable(&self) -> &str {
        match self {
            ZigCompiler::Zig(path) => path,
        }
    }

    pub fn name(&self) -> &str {
        match self {
            ZigCompiler::Zig(_) => "zig",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zig_toolchain_detection() {
        let result = ZigToolchain::detect();
        if let Ok(toolchain) = result {
            assert!(!toolchain.executable.is_empty());
            assert!(!toolchain.zig_version.is_empty());
        }
    }

    #[test]
    fn test_zig_compiler_detection() {
        let result = ZigCompiler::detect();
        if let Ok(compiler) = result {
            assert_eq!(compiler.name(), "zig");
        }
    }
}
