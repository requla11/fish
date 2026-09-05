use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZigImportKind {
    StandardLibrary,
    ExternalPackage(String),
    LocalFile(PathBuf),
}

#[derive(Debug, Clone, Default)]
pub struct ZigDependencyGraph {
    pub file_imports: HashMap<PathBuf, Vec<ZigImportKind>>,
}

impl ZigDependencyGraph {
    pub fn scan_directory<P: AsRef<Path>>(dir: P) -> Self {
        let mut graph = Self::default();
        let mut zig_files = Vec::new();
        collect_zig_files(dir.as_ref(), &mut zig_files);

        for file in zig_files {
            if let Ok(content) = fs::read_to_string(&file) {
                let imports = parse_imports(&content, &file);
                graph.file_imports.insert(file, imports);
            }
        }

        graph
    }

    pub fn direct_dependencies_of(&self, file: &Path) -> Vec<PathBuf> {
        let mut result = Vec::new();
        if let Some(imports) = self.file_imports.get(file) {
            for imp in imports {
                if let ZigImportKind::LocalFile(path) = imp {
                    result.push(path.clone());
                }
            }
        }
        result
    }

    pub fn transitive_dependencies_of(&self, root: &Path) -> HashSet<PathBuf> {
        let mut visited = HashSet::new();
        let mut stack = vec![root.to_path_buf()];

        while let Some(current) = stack.pop() {
            if visited.insert(current.clone()) {
                for dep in self.direct_dependencies_of(&current) {
                    if !visited.contains(&dep) {
                        stack.push(dep);
                    }
                }
            }
        }

        visited.remove(root);
        visited
    }
}

fn collect_zig_files(dir: &Path, acc: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_zig_files(&path, acc);
            } else if path.extension().is_some_and(|ext| ext == "zig") {
                acc.push(path);
            }
        }
    }
}

fn parse_imports(source: &str, current_file: &Path) -> Vec<ZigImportKind> {
    let mut imports = Vec::new();
    let parent = current_file.parent().unwrap_or_else(|| Path::new("."));

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(start) = trimmed.find(r#"@import(""#) {
            let rest = &trimmed[start + 9..];
            if let Some(end) = rest.find(r#"")"#) {
                let target = &rest[..end];
                if target == "std" || target == "builtin" {
                    imports.push(ZigImportKind::StandardLibrary);
                } else if target.ends_with(".zig") {
                    let local_path = parent.join(target);
                    imports.push(ZigImportKind::LocalFile(local_path));
                } else {
                    imports.push(ZigImportKind::ExternalPackage(target.to_string()));
                }
            }
        }
    }

    imports
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_zig_source_import_scanner() {
        let temp = tempdir().unwrap();
        let main_zig = temp.path().join("main.zig");
        let helper_zig = temp.path().join("helper.zig");

        fs::write(
            &main_zig,
            r#"
const std = @import("std");
const helper = @import("helper.zig");
const zap = @import("zap");
pub fn main() void {}
"#,
        )
        .unwrap();

        fs::write(&helper_zig, "pub fn help() void {}").unwrap();

        let graph = ZigDependencyGraph::scan_directory(temp.path());
        assert!(graph.file_imports.contains_key(&main_zig));

        let deps = graph.direct_dependencies_of(&main_zig);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], helper_zig);

        let trans = graph.transitive_dependencies_of(&main_zig);
        assert!(trans.contains(&helper_zig));
    }
}
