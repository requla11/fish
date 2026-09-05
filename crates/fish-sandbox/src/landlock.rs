use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandlockAccessFs(pub u32);

impl LandlockAccessFs {
    pub const EXECUTE: u32 = 1 << 0;
    pub const WRITE_FILE: u32 = 1 << 1;
    pub const READ_FILE: u32 = 1 << 2;
    pub const READ_DIR: u32 = 1 << 3;
    pub const REMOVE_DIR: u32 = 1 << 4;
    pub const REMOVE_FILE: u32 = 1 << 5;
    pub const MAKE_CHAR: u32 = 1 << 6;
    pub const MAKE_DIR: u32 = 1 << 7;
    pub const MAKE_REG: u32 = 1 << 8;
    pub const MAKE_SOCK: u32 = 1 << 9;
    pub const MAKE_FIFO: u32 = 1 << 10;
    pub const MAKE_BLOCK: u32 = 1 << 11;
    pub const MAKE_SYM: u32 = 1 << 12;
    pub const REFER: u32 = 1 << 13;
    pub const TRUNCATE: u32 = 1 << 14;

    pub fn read_only() -> Self {
        Self(Self::EXECUTE | Self::READ_FILE | Self::READ_DIR)
    }

    pub fn read_write() -> Self {
        Self(
            Self::EXECUTE
                | Self::READ_FILE
                | Self::READ_DIR
                | Self::WRITE_FILE
                | Self::REMOVE_FILE
                | Self::REMOVE_DIR
                | Self::MAKE_DIR
                | Self::MAKE_REG
                | Self::MAKE_SYM
                | Self::TRUNCATE,
        )
    }

    pub fn bits(&self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LandlockPathRule {
    pub path: PathBuf,
    pub access: LandlockAccessFs,
}

#[derive(Debug, Clone, Default)]
pub struct LandlockPolicy {
    pub rules: Vec<LandlockPathRule>,
    pub handled_access_fs: u32,
}

impl LandlockPolicy {
    pub fn new() -> Self {
        let default_handled = LandlockAccessFs::read_write().bits()
            | LandlockAccessFs::REFER
            | LandlockAccessFs::MAKE_CHAR
            | LandlockAccessFs::MAKE_BLOCK
            | LandlockAccessFs::MAKE_FIFO
            | LandlockAccessFs::MAKE_SOCK;

        Self {
            rules: Vec::new(),
            handled_access_fs: default_handled,
        }
    }

    pub fn allow_read<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.rules.push(LandlockPathRule {
            path: path.as_ref().to_path_buf(),
            access: LandlockAccessFs::read_only(),
        });
        self
    }

    pub fn allow_read_write<P: AsRef<Path>>(&mut self, path: P) -> &mut Self {
        self.rules.push(LandlockPathRule {
            path: path.as_ref().to_path_buf(),
            access: LandlockAccessFs::read_write(),
        });
        self
    }

    pub fn for_task(
        workspace_root: &Path,
        system_roots: &[&str],
        writable_outputs: &[PathBuf],
    ) -> Self {
        let mut policy = Self::new();
        policy.allow_read(workspace_root);

        for sys in system_roots {
            policy.allow_read(Path::new(sys));
        }

        for out in writable_outputs {
            policy.allow_read_write(out);
        }

        policy
    }

    pub fn is_path_allowed(&self, target: &Path, write: bool) -> bool {
        let mut best_match: Option<&LandlockPathRule> = None;
        for rule in &self.rules {
            if target.starts_with(&rule.path) {
                match best_match {
                    Some(current) => {
                        if rule.path.components().count() > current.path.components().count() {
                            best_match = Some(rule);
                        }
                    }
                    None => {
                        best_match = Some(rule);
                    }
                }
            }
        }

        if let Some(rule) = best_match {
            if write {
                (rule.access.bits() & LandlockAccessFs::WRITE_FILE) != 0
            } else {
                (rule.access.bits() & LandlockAccessFs::READ_FILE) != 0
            }
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_landlock_policy_rule_enforcement() {
        let ws = PathBuf::from("/workspace");
        let sys = ["/usr", "/lib", "/bin"];
        let out = [PathBuf::from("/workspace/target/out")];

        let policy = LandlockPolicy::for_task(&ws, &sys, &out);

        assert!(policy.is_path_allowed(Path::new("/workspace/src/lib.rs"), false));
        assert!(!policy.is_path_allowed(Path::new("/workspace/src/lib.rs"), true));

        assert!(policy.is_path_allowed(Path::new("/usr/bin/gcc"), false));
        assert!(!policy.is_path_allowed(Path::new("/usr/bin/gcc"), true));

        assert!(policy.is_path_allowed(Path::new("/workspace/target/out/app.bin"), true));
        assert!(policy.is_path_allowed(Path::new("/workspace/target/out/app.bin"), false));

        assert!(!policy.is_path_allowed(Path::new("/root/.ssh/id_rsa"), false));
        assert!(!policy.is_path_allowed(Path::new("/etc/shadow"), false));
    }
}
