//! File-event recording and hermeticity analysis.
//!
//! This module compares *observed* filesystem accesses against a task's
//! declared inputs/outputs. Events arrive through
//! [FileEventRecorder::record_access]; attaching an automatic capture
//! source (eBPF tracepoints, strace, platform APIs) is deliberately out of
//! scope until a real implementation lands.

use std::collections::BTreeSet;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAccessType {
    Read,
    Write,
    Execute,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TracedFileEvent {
    pub process_id: u32,
    pub path: PathBuf,
    pub access_type: FileAccessType,
}

/// Summary comparing recorded file events against declared inputs/outputs.
#[derive(Debug, Clone, Default)]
pub struct HermeticitySummary {
    pub declared_inputs: BTreeSet<PathBuf>,
    pub undeclared_inputs: BTreeSet<PathBuf>,
    pub declared_outputs: BTreeSet<PathBuf>,
    pub undeclared_outputs: BTreeSet<PathBuf>,
}

/// Manual file-event recorder feeding hermeticity analysis.
///
/// Callers push observed [TracedFileEvent]s via [Self::record_access];
/// nothing here attaches to the kernel. Loading an actual eBPF program to
/// capture syscalls automatically remains future work.
pub struct FileEventRecorder {
    enabled: bool,
    events: Vec<TracedFileEvent>,
}

impl FileEventRecorder {
    pub fn new() -> Self {
        Self {
            enabled: cfg!(target_os = "linux"),
            events: Vec::new(),
        }
    }

    pub fn is_supported() -> bool {
        cfg!(target_os = "linux")
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn record_access(&mut self, pid: u32, path: PathBuf, access_type: FileAccessType) {
        self.events.push(TracedFileEvent {
            process_id: pid,
            path,
            access_type,
        });
    }

    pub fn analyze_hermeticity(
        &self,
        declared_inputs: &[PathBuf],
        declared_outputs: &[PathBuf],
    ) -> HermeticitySummary {
        let decl_inputs_set: BTreeSet<_> = declared_inputs.iter().cloned().collect();
        let decl_outputs_set: BTreeSet<_> = declared_outputs.iter().cloned().collect();

        let mut summary = HermeticitySummary {
            declared_inputs: decl_inputs_set.clone(),
            declared_outputs: decl_outputs_set.clone(),
            undeclared_inputs: BTreeSet::new(),
            undeclared_outputs: BTreeSet::new(),
        };

        for ev in &self.events {
            match ev.access_type {
                FileAccessType::Read => {
                    if !decl_inputs_set.contains(&ev.path) {
                        summary.undeclared_inputs.insert(ev.path.clone());
                    }
                }
                FileAccessType::Write => {
                    if !decl_outputs_set.contains(&ev.path) {
                        summary.undeclared_outputs.insert(ev.path.clone());
                    }
                }
                FileAccessType::Execute => {}
            }
        }

        summary
    }

    pub fn filter_system_paths(&self, paths: &BTreeSet<PathBuf>) -> BTreeSet<PathBuf> {
        let system_prefixes = [
            "/usr",
            "/lib",
            "/lib64",
            "/proc",
            "/sys",
            "/dev",
            "/etc/ld.so",
            "C:\\Windows",
            "C:\\Program Files",
        ];

        paths
            .iter()
            .filter(|p| {
                let s = p.to_string_lossy();
                !system_prefixes.iter().any(|prefix| s.starts_with(prefix))
            })
            .cloned()
            .collect()
    }

    pub fn discover_dynamic_dependencies(&self, root_dir: &std::path::Path) -> BTreeSet<PathBuf> {
        let mut deps = BTreeSet::new();
        for ev in &self.events {
            if matches!(ev.access_type, FileAccessType::Read) && ev.path.starts_with(root_dir) {
                deps.insert(ev.path.clone());
            }
        }
        deps
    }
}

impl Default for FileEventRecorder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_undeclared_dependency_detection() {
        let mut tracer = FileEventRecorder::new();
        let p_declared = PathBuf::from("/workspace/src/lib.rs");
        let p_undeclared = PathBuf::from("/etc/secret_config.json");
        let p_output = PathBuf::from("/workspace/target/out.o");

        tracer.record_access(1001, p_declared.clone(), FileAccessType::Read);
        tracer.record_access(1001, p_undeclared.clone(), FileAccessType::Read);
        tracer.record_access(1001, p_output.clone(), FileAccessType::Write);

        let summary = tracer.analyze_hermeticity(&[p_declared], &[p_output]);
        assert!(summary.undeclared_inputs.contains(&p_undeclared));
        assert_eq!(summary.undeclared_outputs.len(), 0);
    }

    #[test]
    fn test_filter_system_paths_and_discovery() {
        let mut tracer = FileEventRecorder::new();
        let root = PathBuf::from("/workspace");
        let project_file = root.join("src/main.rs");
        let sys_file = PathBuf::from("/usr/include/stdio.h");

        tracer.record_access(2002, project_file.clone(), FileAccessType::Read);
        tracer.record_access(2002, sys_file.clone(), FileAccessType::Read);

        let discovered = tracer.discover_dynamic_dependencies(&root);
        assert_eq!(discovered.len(), 1);
        assert!(discovered.contains(&project_file));

        let mut all_paths = BTreeSet::new();
        all_paths.insert(project_file.clone());
        all_paths.insert(sys_file);

        let filtered = tracer.filter_system_paths(&all_paths);
        assert_eq!(filtered.len(), 1);
        assert!(filtered.contains(&project_file));
    }
}
