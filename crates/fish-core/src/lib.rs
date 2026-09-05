#![forbid(unsafe_code)]

pub mod adapters;
pub mod banana_ast;
pub use adapters::{DetectedMonorepoKind, MonorepoDiscoveryResult, ZeroConfigAdapter};
pub use banana_ast::FishAstService;

pub mod backend;
pub mod backend_utils;
pub mod boundary;
pub mod compile_commands;
pub mod config;
pub mod diagnostics;
pub mod drift;
pub mod environment;
pub mod error;
pub mod input_filter;
pub mod profiling;
pub mod project;
pub mod proto;
pub mod remediation;
pub mod sandbox_profiles;
pub mod security;
pub mod testing;
pub mod toolchain;
pub mod toolchain_downloader;
pub mod toolchain_lock;

pub use remediation::{AutoRemediator, CompilerDiagnostic, ErrorKind, RemediationPatch};

#[cfg(windows)]
pub mod windows_compat;

pub use backend::BuildBackend;
pub use backend_utils::{
    BinaryUtils, DEFAULT_EXCLUDED_DIRS, FingerprintUtils, TaskDagBuilder, ToolchainUtils,
};
pub use boundary::{
    BoundaryEnforcer, BoundaryTagRule, BoundaryViolation, BoundaryViolationKind,
    PackageBoundaryMeta,
};
pub use compile_commands::{CompilationDatabase, CompileCommand};
pub use config::{
    BuildConfig, CIConfig, CacheConfig, ConfigError, ExperimentalConfig, FishConfig, GeneralConfig,
    SecurityConfig,
};
pub use diagnostics::{
    ComponentHealthMonitor, DiagnosticLog, DiagnosticLogger, DiagnosticReport, HealthCheckRegistry,
    LogLevel, OpPerformanceMetrics, SystemDiagnostics,
};
pub use environment::{EnvironmentFingerprint, is_offline, set_offline};
pub use error::{
    DiagnosticInfo, ErrorContext, ErrorSeverity, FishError, HealthCheck, HealthCheckResult,
    HealthStatus, RecoveryStrategy, Result,
};
pub use input_filter::MicroInputFilter;
pub use profiling::{
    MemoryTracker, PerformanceMetrics, Profiler, ResourceMonitor, ResourceUsage, TaskProfile,
};
pub use security::{
    InputValidator, SecurityError, SecurityLevel, SecurityPolicy, SecurityValidator,
};
pub use testing::{
    Benchmark, BenchmarkResult, IntegrationTestEnvironment, ParallelTestRunner, PropertyTestResult,
};
pub use toolchain::find_executable_in_path;
pub use toolchain::{ToolchainKind, ToolchainRegistry, ToolchainSpec};
pub use toolchain_downloader::{RemoteToolchainSource, ToolchainDownloader};

#[cfg(windows)]
pub use windows_compat::{
    get_windows_version, is_developer_mode_enabled, is_file_locked, safe_replace_file,
    try_symlink_or_copy,
};
