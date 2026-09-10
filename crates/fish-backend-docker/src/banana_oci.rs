use banana::oci::{OciBuilder, OciImageResult};
use std::path::Path;

pub struct FishOciCompiler;

impl FishOciCompiler {
    pub fn compile_rootfs_to_oci(
        rootfs: &Path,
        output_tar: &Path,
        entrypoint: Vec<String>,
        working_dir: &str,
    ) -> Result<OciImageResult, anyhow::Error> {
        let builder = OciBuilder::new()
            .entrypoint(entrypoint)
            .working_dir(working_dir);

        builder.build_from_rootfs(rootfs, output_tar)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_fish_oci_compiler_integration() {
        let temp = tempdir().unwrap();
        let rootfs = temp.path().join("rootfs");
        fs::create_dir_all(rootfs.join("bin")).unwrap();
        fs::write(rootfs.join("bin/app"), b"elf-payload").unwrap();

        let out_tar = temp.path().join("app.tar");
        let res = FishOciCompiler::compile_rootfs_to_oci(
            &rootfs,
            &out_tar,
            vec!["/bin/app".to_string()],
            "/",
        )
        .unwrap();

        assert!(out_tar.exists());
        assert!(res.manifest_digest.starts_with("sha256:"));
    }
}
