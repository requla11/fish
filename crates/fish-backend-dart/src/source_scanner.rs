use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DartImportKind {
    CoreSdk(String),
    Package(String),
    RelativeFile(PathBuf),
    PartFile(PathBuf),
}

#[derive(Debug, Clone, Default)]
pub struct DartSourceGraph {
    pub file_imports: HashMap<PathBuf, Vec<DartImportKind>>,
}

impl DartSourceGraph {
    pub fn scan_directory<P: AsRef<Path>>(dir: P) -> Self {
        let mut graph = Self::default();
        let mut dart_files = Vec::new();
        collect_dart_files(dir.as_ref(), &mut dart_files);

        for file in dart_files {
            if let Ok(content) = fs::read_to_string(&file) {
                let imports = parse_dart_imports(&content, &file);
                graph.file_imports.insert(file, imports);
            }
        }

        graph
    }

    pub fn direct_relative_dependencies_of(&self, file: &Path) -> Vec<PathBuf> {
        let mut result = Vec::new();
        if let Some(imports) = self.file_imports.get(file) {
            for imp in imports {
                match imp {
                    DartImportKind::RelativeFile(p) | DartImportKind::PartFile(p) => {
                        result.push(p.clone());
                    }
                    _ => {}
                }
            }
        }
        result
    }
}

fn collect_dart_files(dir: &Path, acc: &mut Vec<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_dart_files(&path, acc);
            } else if path.extension().is_some_and(|ext| ext == "dart") {
                acc.push(path);
            }
        }
    }
}

fn parse_dart_imports(source: &str, current_file: &Path) -> Vec<DartImportKind> {
    let mut imports = Vec::new();
    let parent = current_file.parent().unwrap_or_else(|| Path::new("."));

    for line in source.lines() {
        let trimmed = line.trim();
        if (trimmed.starts_with("import ") || trimmed.starts_with("part "))
            && let Some(quote_start) = trimmed.find('\'').or_else(|| trimmed.find('"'))
        {
            let quote_char = trimmed.as_bytes()[quote_start] as char;
            let rest = &trimmed[quote_start + 1..];
            if let Some(quote_end) = rest.find(quote_char) {
                let uri = &rest[..quote_end];
                if uri.starts_with("dart:") {
                    imports.push(DartImportKind::CoreSdk(uri.to_string()));
                } else if uri.starts_with("package:") {
                    imports.push(DartImportKind::Package(uri.to_string()));
                } else if trimmed.starts_with("part ") {
                    imports.push(DartImportKind::PartFile(parent.join(uri)));
                } else {
                    imports.push(DartImportKind::RelativeFile(parent.join(uri)));
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
    fn test_dart_source_import_scanner() {
        let temp = tempdir().unwrap();
        let main_dart = temp.path().join("main.dart");
        let helper_dart = temp.path().join("helper.dart");

        fs::write(
            &main_dart,
            r#"
import 'dart:async';
import 'package:flutter/material.dart';
import 'helper.dart';

void main() {}
"#,
        )
        .unwrap();

        fs::write(&helper_dart, "void help() {}").unwrap();

        let graph = DartSourceGraph::scan_directory(temp.path());
        assert!(graph.file_imports.contains_key(&main_dart));

        let deps = graph.direct_relative_dependencies_of(&main_dart);
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0], helper_dart);
    }
}
