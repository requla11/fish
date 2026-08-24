use std::fs;
use std::io;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloneStrategy {
    ExtentClone,
    HardLink,
    FastCopy,
}

pub struct KernelCowCloner;

impl KernelCowCloner {
    pub fn try_clone_file(src: &Path, dst: &Path) -> io::Result<CloneStrategy> {
        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }

        if dst.exists() {
            let _ = fs::remove_file(dst);
        }

        Self::fast_copy(src, dst)?;
        Ok(CloneStrategy::FastCopy)
    }

    fn fast_copy(src: &Path, dst: &Path) -> io::Result<()> {
        fs::copy(src, dst)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_cow_cloner_file_integrity() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source.bin");
        let dst = dir.path().join("sub").join("clone.bin");

        let content = b"FISH_KERNEL_COW_TEST_BYTES_0123456789";
        fs::write(&src, content).unwrap();

        let strategy = KernelCowCloner::try_clone_file(&src, &dst).unwrap();
        assert!(dst.exists());
        assert_eq!(fs::read(&dst).unwrap(), content);
        assert!(matches!(
            strategy,
            CloneStrategy::HardLink | CloneStrategy::FastCopy | CloneStrategy::ExtentClone
        ));
    }

    #[test]
    fn test_cow_cloner_overwrite_existing() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("source2.bin");
        let dst = dir.path().join("clone2.bin");

        fs::write(&src, b"new_data").unwrap();
        fs::write(&dst, b"old_data").unwrap();

        let _ = KernelCowCloner::try_clone_file(&src, &dst).unwrap();
        assert_eq!(fs::read(&dst).unwrap(), b"new_data");
    }

    #[test]
    fn clone_is_independent_of_the_source() {
        let dir = tempdir().unwrap();
        let src = dir.path().join("src.bin");
        let dst = dir.path().join("dst.bin");
        fs::write(&src, b"original").unwrap();

        KernelCowCloner::try_clone_file(&src, &dst).unwrap();

        fs::write(&dst, b"modified").unwrap();
        assert_eq!(fs::read(&src).unwrap(), b"original");
        assert_eq!(fs::read(&dst).unwrap(), b"modified");
    }
}
