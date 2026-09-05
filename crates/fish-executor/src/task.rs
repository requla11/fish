use std::path::PathBuf;
use std::time::Duration;

use crate::command::CommandSpec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheEntry {
    pub key: String,

    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRequirements {
    pub permits: usize,
    pub tokens: Vec<String>,
    pub exclusive: bool,
}

impl Default for ResourceRequirements {
    fn default() -> Self {
        Self {
            permits: 1,
            tokens: Vec::new(),
            exclusive: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub label: String,

    pub description: String,

    pub spec: CommandSpec,

    pub cache: Option<CacheEntry>,

    pub artifacts: Vec<PathBuf>,

    pub inputs: Vec<PathBuf>,

    pub resources: ResourceRequirements,
}

impl Task {
    pub fn new(
        label: impl Into<String>,
        description: impl Into<String>,
        spec: CommandSpec,
    ) -> Self {
        Self {
            label: label.into(),
            description: description.into(),
            spec,
            cache: None,
            artifacts: Vec::new(),
            inputs: Vec::new(),
            resources: ResourceRequirements::default(),
        }
    }

    pub fn with_cache(mut self, entry: CacheEntry) -> Self {
        self.cache = Some(entry);
        self
    }

    pub fn with_artifacts(mut self, artifacts: Vec<PathBuf>) -> Self {
        self.artifacts = artifacts;
        self
    }

    pub fn with_inputs(mut self, inputs: Vec<PathBuf>) -> Self {
        self.inputs = inputs;
        self
    }

    pub fn with_resources(mut self, resources: ResourceRequirements) -> Self {
        self.resources = resources;
        self
    }

    pub fn with_permits(mut self, permits: usize) -> Self {
        self.resources.permits = permits;
        self
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.resources.tokens.push(token.into());
        self
    }

    pub fn with_exclusive(mut self, exclusive: bool) -> Self {
        self.resources.exclusive = exclusive;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskStatus {
    Executed,

    Cached,

    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskOutcome {
    pub status: TaskStatus,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub duration: Duration,
}

impl TaskOutcome {
    pub fn executed(_task: &Task) -> Self {
        Self {
            status: TaskStatus::Executed,
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            duration: Duration::ZERO,
        }
    }

    pub fn cached(_task: &Task) -> Self {
        Self {
            status: TaskStatus::Cached,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            duration: Duration::ZERO,
        }
    }

    pub fn failed(_task: &Task, message: impl Into<String>) -> Self {
        Self {
            status: TaskStatus::Failed,
            exit_code: None,
            stdout: String::new(),
            stderr: message.into(),
            duration: Duration::ZERO,
        }
    }
}
