use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SandboxPlatform {
    LinuxBubblewrap,
    LinuxLandlock,
    MacOSSandboxExec,
    WindowsJobObject,
    AppleDaemon,
    FallbackRestricted,
}

#[derive(Debug, Clone)]
pub struct HermeticProcessSandbox {
    pub platform: SandboxPlatform,
    pub root_dir: PathBuf,
    pub writable_dirs: Vec<PathBuf>,
    pub allow_network: bool,
    pub max_memory_bytes: u64,
}

impl HermeticProcessSandbox {
    pub fn auto_configure(root_dir: PathBuf, out_dir: PathBuf) -> Self {
        let platform = if cfg!(target_os = "linux") {
            SandboxPlatform::LinuxBubblewrap
        } else if cfg!(target_os = "macos") {
            SandboxPlatform::MacOSSandboxExec
        } else if cfg!(target_os = "windows") {
            SandboxPlatform::WindowsJobObject
        } else {
            SandboxPlatform::FallbackRestricted
        };

        Self {
            platform,
            root_dir,
            writable_dirs: vec![out_dir, std::env::temp_dir()],
            allow_network: false,
            max_memory_bytes: 4 * 1024 * 1024 * 1024,
        }
    }

    pub fn wrap_command_args(&self, executable: &str, args: &[String]) -> (String, Vec<String>) {
        match self.platform {
            SandboxPlatform::AppleDaemon => {
                let mut apple_args = vec![
                    "run".to_string(),
                    "--offline".to_string(),
                    "--".to_string(),
                    executable.to_string(),
                ];
                apple_args.extend_from_slice(args);
                ("apple".to_string(), apple_args)
            }
            SandboxPlatform::LinuxBubblewrap => {
                let mut bwrap_args = vec!["--unshare-all".to_string()];
                if !self.allow_network {
                    bwrap_args.push("--unshare-net".to_string());
                }
                bwrap_args.extend_from_slice(&[
                    "--ro-bind".to_string(),
                    "/usr".to_string(),
                    "/usr".to_string(),
                    "--ro-bind".to_string(),
                    "/bin".to_string(),
                    "/bin".to_string(),
                    "--ro-bind".to_string(),
                    "/lib".to_string(),
                    "/lib".to_string(),
                    "--dev".to_string(),
                    "/dev".to_string(),
                    "--proc".to_string(),
                    "/proc".to_string(),
                    "--ro-bind".to_string(),
                    self.root_dir.to_string_lossy().to_string(),
                    self.root_dir.to_string_lossy().to_string(),
                ]);
                if std::path::Path::new("/lib64").exists() {
                    bwrap_args.extend_from_slice(&[
                        "--ro-bind".to_string(),
                        "/lib64".to_string(),
                        "/lib64".to_string(),
                    ]);
                }
                if std::path::Path::new("/etc").exists() {
                    bwrap_args.extend_from_slice(&[
                        "--ro-bind".to_string(),
                        "/etc".to_string(),
                        "/etc".to_string(),
                    ]);
                }
                let exec_p = std::path::Path::new(executable);
                if let Some(parent) = exec_p.parent() {
                    if parent.exists()
                        && !parent.starts_with("/bin")
                        && !parent.starts_with("/usr")
                        && !parent.starts_with("/lib")
                    {
                        bwrap_args.extend_from_slice(&[
                            "--ro-bind".to_string(),
                            parent.to_string_lossy().to_string(),
                            parent.to_string_lossy().to_string(),
                        ]);
                    }
                }
                if let Ok(home) = std::env::var("HOME") {
                    let home_path = std::path::Path::new(&home);
                    let rustup = home_path.join(".rustup");
                    if rustup.exists() {
                        bwrap_args.extend_from_slice(&[
                            "--ro-bind".to_string(),
                            rustup.to_string_lossy().to_string(),
                            rustup.to_string_lossy().to_string(),
                        ]);
                    }
                    let cargo = home_path.join(".cargo");
                    if cargo.exists() {
                        bwrap_args.extend_from_slice(&[
                            "--bind".to_string(),
                            cargo.to_string_lossy().to_string(),
                            cargo.to_string_lossy().to_string(),
                        ]);
                    }
                }
                if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
                    let p = std::path::Path::new(&cargo_home);
                    if p.exists() {
                        bwrap_args.extend_from_slice(&[
                            "--bind".to_string(),
                            p.to_string_lossy().to_string(),
                            p.to_string_lossy().to_string(),
                        ]);
                    }
                }
                if let Ok(rustup_home) = std::env::var("RUSTUP_HOME") {
                    let p = std::path::Path::new(&rustup_home);
                    if p.exists() {
                        bwrap_args.extend_from_slice(&[
                            "--ro-bind".to_string(),
                            p.to_string_lossy().to_string(),
                            p.to_string_lossy().to_string(),
                        ]);
                    }
                }
                for w in &self.writable_dirs {
                    bwrap_args.push("--bind".to_string());
                    bwrap_args.push(w.to_string_lossy().to_string());
                    bwrap_args.push(w.to_string_lossy().to_string());
                }
                bwrap_args.push(executable.to_string());
                bwrap_args.extend_from_slice(args);
                ("bwrap".to_string(), bwrap_args)
            }
            SandboxPlatform::MacOSSandboxExec => {
                let mut profile = String::from(
                    "(version 1)(deny default)(allow process*)(allow sysctl-read)(allow file-read*)",
                );
                profile.push_str(&format!(
                    "(allow file-read*(subpath \"{}\"))",
                    self.root_dir.to_string_lossy()
                ));
                for w in &self.writable_dirs {
                    profile.push_str(&format!(
                        "(allow file-write*(subpath \"{}\"))",
                        w.to_string_lossy()
                    ));
                }
                profile.push_str("(allow file-write*(subpath \"/dev\"))");
                profile.push_str("(allow file-write*(subpath \"/private/tmp\"))");
                profile.push_str("(allow file-write*(subpath \"/private/var\"))");
                if self.allow_network {
                    profile.push_str("(allow network*)");
                }
                let mut sb_args = vec!["-p".to_string(), profile, executable.to_string()];
                sb_args.extend_from_slice(args);
                ("sandbox-exec".to_string(), sb_args)
            }
            SandboxPlatform::LinuxLandlock
            | SandboxPlatform::WindowsJobObject
            | SandboxPlatform::FallbackRestricted => (executable.to_string(), args.to_vec()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hermetic_sandbox_wrapping() {
        let sb = HermeticProcessSandbox {
            platform: SandboxPlatform::LinuxBubblewrap,
            root_dir: PathBuf::from("/workspace"),
            writable_dirs: vec![PathBuf::from("/workspace/target")],
            allow_network: false,
            max_memory_bytes: 1024 * 1024,
        };

        let (cmd, args) = sb.wrap_command_args("gcc", &["-c".to_string(), "main.c".to_string()]);
        assert_eq!(cmd, "bwrap");
        assert!(args.contains(&"--unshare-all".to_string()));
        assert!(args.contains(&"--unshare-net".to_string()));
        assert!(args.contains(&"/workspace".to_string()));
        assert!(args.contains(&"gcc".to_string()));
    }

    #[test]
    fn test_macos_sandbox_exec_wrapping() {
        let sb = HermeticProcessSandbox {
            platform: SandboxPlatform::MacOSSandboxExec,
            root_dir: PathBuf::from("/workspace"),
            writable_dirs: vec![PathBuf::from("/workspace/target")],
            allow_network: false,
            max_memory_bytes: 1024 * 1024,
        };

        let (cmd, args) = sb.wrap_command_args("clang", &["-c".to_string(), "main.c".to_string()]);
        assert_eq!(cmd, "sandbox-exec");
        assert!(args[1].contains("(version 1)"));
        assert!(args[1].contains("(subpath \"/workspace\")"));
        assert!(args[1].contains("(allow file-write*(subpath \"/workspace/target\"))"));
    }

    #[test]
    fn test_apple_daemon_sandbox_wrapping() {
        let sb = HermeticProcessSandbox {
            platform: SandboxPlatform::AppleDaemon,
            root_dir: PathBuf::from("/workspace"),
            writable_dirs: vec![PathBuf::from("/workspace/target")],
            allow_network: false,
            max_memory_bytes: 1024 * 1024,
        };

        let (cmd, args) = sb.wrap_command_args("rustc", &["main.rs".to_string()]);
        assert_eq!(cmd, "apple");
        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--offline".to_string()));
        assert!(args.contains(&"rustc".to_string()));
        assert!(args.contains(&"main.rs".to_string()));
    }
}
