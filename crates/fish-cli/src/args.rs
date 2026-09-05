#![forbid(unsafe_code)]

//! Command-line argument definitions for fish CLI
//!
//! This module contains all argument structures and enums for CLI subcommands.

use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

use crate::watch;
use fish_backend_rust::BuildMode;

/// CLI entry point
#[derive(Debug, Parser)]
#[command(
    name = "fish",
    version = env!("CARGO_PKG_VERSION"),
    about = "Fish: a fast, cache-first build orchestration system for Rust and beyond.",
    long_about = None
)]
pub struct Cli {
    /// Enable experimental features (use at your own risk)
    #[arg(long, global = true)]
    pub experimental: bool,

    /// Disallow network access and fail-fast on remote operations
    #[arg(long, global = true)]
    pub offline: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// CLI subcommands
#[derive(Debug, Subcommand)]
pub enum Command {
    Version,
    Init(InitArgs),
    New(NewArgs),
    Build(BuildArgs),
    Check(CheckArgs),
    Test(TestArgs),
    Clean(CleanArgs),
    Run(RunArgs),
    Graph(GraphArgs),
    #[command(alias = "dev")]
    Watch(WatchArgs),
    CacheServer(CacheServerArgs),
    Worker(WorkerArgs),
    Affected(AffectedArgs),
    Doctor(DoctorArgs),
    Cache(CacheArgs),
    Ci(CiArgs),
    History(HistoryArgs),
    Rewind(RewindArgs),
    Attest(AttestArgs),
    Verify(VerifyArgs),
    #[command(hide = true)]
    LivePatch(LivePatchArgs),
    #[command(hide = true)]
    Jit(JitArgs),
    #[command(hide = true)]
    SuperOpt(SuperOptArgs),
    Plugin(PluginArgs),
    Fix(FixArgs),
    SigningKey,
    Heal(HealArgs),
    #[command(hide = true)]
    GenDocs(GenDocsArgs),
    CostEstimate(CostEstimateArgs),
    #[command(alias = "dashboard")]
    Ui(UiArgs),
    Query(QueryArgs),
    Daemon(DaemonArgs),
    #[command(hide = true)]
    Ai(AiArgs),
    #[command(hide = true)]
    Lsp(LspArgs),
    Why(WhyArgs),
    #[command(hide = true)]
    Pash(PashArgs),
    #[command(hide = true)]
    Qpc(QpcArgs),
}

#[derive(Debug, Args)]
pub struct LspArgs {
    #[arg(long)]
    pub stdio: bool,
}

#[derive(Debug, Args)]
pub struct FixArgs {
    #[arg(long, short)]
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub apply: bool,
    #[arg(long)]
    pub diff: bool,
    #[arg(long)]
    pub ai: bool,
}

/// Arguments for `fish heal`.
#[derive(Debug, Args)]
pub struct HealArgs {
    /// Project directory (defaults to cwd).
    #[arg(long, short)]
    pub path: Option<PathBuf>,
    /// How many recent commits to scan.
    #[arg(long, default_value_t = 10)]
    pub depth: usize,
    /// Build command words to test at each commit (default: cargo build).
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub cmd: Vec<String>,
}

/// Arguments for the hidden docs generator.
#[derive(Debug, Args)]
pub struct GenDocsArgs {
    /// Output file (defaults to docs/cli-reference.md).
    #[arg(long, short)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct CostEstimateArgs {
    /// Inline workload: comma-separated `label=seconds` pairs.
    #[arg(long, conflicts_with = "tasks_json")]
    pub durations: Option<String>,
    /// JSON file with `{"tasks":[{"label":"...","duration_secs":1.0}]}`.
    #[arg(long, conflicts_with = "durations")]
    pub tasks_json: Option<PathBuf>,
    /// Comma-separated providers to price (defaults to every provider in the catalog).
    #[arg(long)]
    pub providers: Option<String>,
    /// Specific instance type to price instead of each provider's cheapest.
    #[arg(long)]
    pub instance: Option<String>,
    /// Number of concurrent build jobs the cloud fleet must sustain. [default: 8]
    #[arg(long, default_value_t = 8)]
    pub parallelism: usize,
    /// Artifact download volume per run, in GB (billed at egress rates).
    #[arg(long, default_value_t = 0.0)]
    pub egress_gb: f64,
    /// Cache footprint stored in the cloud, in GB.
    #[arg(long, default_value_t = 0.0)]
    pub storage_gb: f64,
    /// Months the cache storage is retained when computing storage cost. [default: 1]
    #[arg(long, default_value_t = 1)]
    pub retention_months: u32,
    /// Task labels already served from cache; excluded from compute pricing.
    #[arg(long = "cached")]
    pub cached: Vec<String>,
    /// Custom TOML pricing catalog overriding the embedded defaults.
    #[arg(long)]
    pub pricing_file: Option<PathBuf>,
    /// Emit the machine-readable JSON report.
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct UiArgs {
    #[arg(long, default_value_t = 4000)]
    pub port: u16,
    #[arg(long)]
    pub open: bool,
    #[arg(long, short)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long, short)]
    pub path: Option<PathBuf>,
    #[arg(long, short)]
    pub force: bool,
    /// Plain-language project description, e.g. --describe "rust cli + python tools".
    #[arg(long)]
    pub describe: Option<String>,
}

#[derive(Debug, Args)]
pub struct NewArgs {
    pub name: String,
    #[arg(long, short)]
    pub template: Option<String>,
    #[arg(long, short)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    #[arg(long)]
    pub ai: bool,
    #[arg(long)]
    pub fix: bool,
}

/// Arguments for affected command
#[derive(Debug, Args)]
pub struct AffectedArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    /// The git revision (commit/tag/branch) to diff against. `HEAD` compares
    /// the working tree, an explicit revision compares the tree of that
    /// revision against the working tree.
    #[arg(long, default_value = "HEAD")]
    pub since: String,
    /// Git base reference to compare against (e.g. `--base origin/main`).
    #[arg(long)]
    pub base: Option<String>,
    #[arg(long, default_value = "build", value_enum)]
    pub mode: AffectedMode,
    /// Output affected packages and PASH status as JSON.
    #[arg(long)]
    pub json: bool,
}

/// Affected build mode
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AffectedMode {
    Build,
    Check,
    Test,
}

impl AffectedMode {
    pub fn to_build_mode(self) -> BuildMode {
        match self {
            AffectedMode::Build => BuildMode::Build,
            AffectedMode::Check => BuildMode::Check,
            AffectedMode::Test => BuildMode::Test,
        }
    }
}

/// Arguments for cache command
#[derive(Debug, Args)]
pub struct CacheArgs {
    /// Cache directory; defaults to `FISH_CACHE_DIR` or `~/.fish/cache`.
    #[arg(long)]
    pub dir: Option<PathBuf>,
    #[command(subcommand)]
    pub command: CacheCommand,
}

/// Cache subcommands
#[derive(Debug, Subcommand)]
pub enum CacheCommand {
    /// Print how many fingerprints/objects the local cache holds and how
    /// much disk space they occupy.
    Stats,
    /// Delete stale fingerprints and oversized cache content.
    #[command(alias = "gc")]
    Prune(CachePruneArgs),
    Verify(CacheVerifyArgs),
    /// CAS operations for artifact storage
    Cas(CasArgs),
}

#[derive(Debug, Args)]
pub struct CacheVerifyArgs {
    #[arg(long, short)]
    pub repair: bool,

    #[arg(long, short)]
    pub verbose: bool,
}

/// Arguments for CAS operations
#[derive(Debug, Args)]
pub struct CasArgs {
    #[command(subcommand)]
    pub command: CasCommand,
}

/// CAS subcommands
#[derive(Debug, Subcommand)]
pub enum CasCommand {
    /// Show CAS storage statistics
    Stats,
    /// Upload an artifact to CAS
    Upload {
        /// Path to the artifact file
        #[arg(value_name = "FILE")]
        file: PathBuf,
        /// Artifact type (e.g., binary, library, docker-image)
        #[arg(long)]
        artifact_type: Option<String>,
        /// Source information (e.g., package name, target)
        #[arg(long)]
        source: Option<String>,
    },
    /// Download an artifact from CAS by hash
    Download {
        /// Artifact hash
        #[arg(value_name = "HASH")]
        hash: String,
        /// Output path for the downloaded artifact
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// List all artifacts in CAS
    List,
    /// Delete an artifact from CAS
    Delete {
        /// Artifact hash
        #[arg(value_name = "HASH")]
        hash: String,
    },
    /// Cleanup old artifacts from CAS
    Cleanup {
        /// Remove artifacts older than this age, e.g. `7d`, `12h`, `30m`
        #[arg(long = "older-than")]
        older_than: Option<String>,
        /// Keep CAS under this size, e.g. `10GB`, `500MB`
        #[arg(long = "max-size")]
        max_size: Option<String>,
    },
}

/// Arguments for CI command
#[derive(Debug, Args)]
pub struct CiArgs {
    #[command(subcommand)]
    pub command: CiCommand,
}

/// CI subcommands
#[derive(Debug, Subcommand)]
pub enum CiCommand {
    /// Initialize CI configuration
    Init {
        /// CI platform (github, gitlab, circleci, bitbucket, all)
        #[arg(long, default_value = "github")]
        platform: String,
        /// Enable cache
        #[arg(long, default_value = "true")]
        cache: bool,
        /// Remote cache URL
        #[arg(long)]
        remote_cache: Option<String>,
    },
    /// Export CI configuration
    Export {
        /// Output file path
        #[arg(short = 'o', long)]
        output: PathBuf,
        /// CI platform (github, gitlab, circleci, bitbucket)
        #[arg(long, default_value = "github")]
        platform: String,
    },
}

/// Arguments for cache prune
#[derive(Debug, Args)]
pub struct CachePruneArgs {
    /// Delete records older than this age, e.g. `7d`, `12h`, `30m`.
    #[arg(long = "older-than")]
    pub older_than: Option<String>,
    /// Keep the cache under this size, e.g. `10GB`, `500MB`. The oldest
    /// entries are removed first.
    #[arg(long = "max-size")]
    pub max_size: Option<String>,
}

/// Arguments for build command
#[derive(Debug, Args)]
pub struct BuildArgs {
    #[command(flatten)]
    pub common: CommonArgs,
}

/// Arguments for check command
#[derive(Debug, Args)]
pub struct CheckArgs {
    #[command(flatten)]
    pub common: CommonArgs,
}

/// Arguments for test command
#[derive(Debug, Args)]
pub struct TestArgs {
    #[command(flatten)]
    pub common: CommonArgs,
}

/// Common arguments shared by build/check/test commands
#[derive(Debug, Args, Clone)]
pub struct CommonArgs {
    pub path: Option<PathBuf>,
    #[arg(short = 'j', long = "jobs")]
    pub jobs: Option<usize>,
    #[arg(short = 'v', long)]
    pub verbose: bool,
    #[arg(long = "no-cache")]
    pub no_cache: bool,
    #[arg(long = "sandbox")]
    pub sandbox: bool,
    /// Run every build task through the `apple` hermetic sandbox (requires
    /// the `apple` binary on PATH).
    #[arg(long = "apple")]
    pub apple: bool,
    #[arg(long = "timeout")]
    pub timeout_secs: Option<u64>,
    #[arg(long = "profile", num_args = 0..=1, default_missing_value = "fish_trace.json")]
    pub profile: Option<PathBuf>,
    #[arg(long = "tui")]
    pub tui: bool,
    #[arg(long = "remote-cache")]
    pub remote_cache: Option<String>,
    #[arg(long = "remote-cache-token")]
    pub remote_cache_token: Option<String>,
    #[arg(long = "remote-workers", value_delimiter = ',')]
    pub remote_workers: Option<Vec<String>>,
    #[arg(long = "remote-workers-token")]
    pub remote_workers_token: Option<String>,
    /// Local cache directory; defaults to `FISH_CACHE_DIR` or `~/.fish/cache`.
    #[arg(long = "cache-dir")]
    pub cache_dir: Option<PathBuf>,
    /// Ship a compressed snapshot of the working tree to remote workers so
    /// they can run tasks without sharing a filesystem.
    #[arg(long = "send-source")]
    pub send_source: bool,
    /// Throttle the build to `jobs / 2` concurrent workers (minimum 1)
    /// whenever the system's available memory drops below this percentage
    /// of total memory.
    #[arg(long = "ram-limit")]
    pub ram_limit: Option<u8>,
    #[arg(long = "semantic")]
    pub semantic: bool,
    #[arg(long = "ramdisk")]
    pub ramdisk: bool,
    #[arg(long = "swarm")]
    pub swarm: bool,
    #[arg(long = "reflink")]
    pub reflink: bool,
    #[arg(long = "hermetic-trace")]
    pub hermetic_trace: bool,
    #[arg(long = "swarm-compute")]
    pub swarm_compute: bool,
    #[arg(long = "critical-path")]
    pub critical_path: bool,
    /// Load a previously saved execution trace and replay it to verify
    /// hermetic determinism before running the actual build.
    #[arg(long = "replay-trace")]
    pub replay_trace: Option<PathBuf>,
    #[arg(long = "turbo-link")]
    pub turbo_link: bool,
    #[arg(long = "speculative")]
    pub speculative: bool,
    #[arg(long = "daemon-pool")]
    pub daemon_pool: bool,
    #[arg(long = "kernel-bypass")]
    pub kernel_bypass: bool,
    #[arg(long = "wasm-sandbox")]
    pub wasm_sandbox: bool,
    #[arg(long = "super-opt")]
    pub super_opt: bool,
    #[arg(long = "explain")]
    pub explain: bool,
    #[arg(long = "summary")]
    pub summary: bool,
    #[arg(long = "summary-file")]
    pub summary_file: Option<PathBuf>,
    #[arg(long = "otel-endpoint")]
    pub otel_endpoint: Option<String>,
    /// Disable automatic cross-language dependency inference between detected
    /// projects (see `cross_deps`); references are then ignored and each
    /// ecosystem builds independently.
    #[arg(long = "no-infer-deps")]
    pub no_infer_deps: bool,
}

#[derive(Debug, Args)]
pub struct QueryArgs {
    pub expr: String,
    #[arg(long, short)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct DaemonArgs {
    #[command(subcommand)]
    pub command: DaemonCommand,
}

#[derive(Debug, Subcommand)]
pub enum DaemonCommand {
    Start {
        #[arg(long, default_value_t = 9527)]
        port: u16,
    },
    Status {
        #[arg(long, default_value_t = 9527)]
        port: u16,
    },
    Stop {
        #[arg(long, default_value_t = 9527)]
        port: u16,
    },
}

/// Arguments for live-patch command
#[derive(Debug, Args)]
pub struct LivePatchArgs {
    pub process_id: u32,
    pub target_binary: PathBuf,
    pub path: Option<PathBuf>,
}

/// Arguments for JIT command
#[derive(Debug, Args)]
pub struct JitArgs {
    pub function_name: String,
    #[arg(default_value = "42")]
    pub value: i32,
}

/// Arguments for super-opt command
#[derive(Debug, Args)]
pub struct SuperOptArgs {
    pub input_file: PathBuf,
    pub output_file: PathBuf,
}

/// Arguments for history command
#[derive(Debug, Args)]
pub struct HistoryArgs {
    pub path: Option<PathBuf>,
}

/// Arguments for rewind command
#[derive(Debug, Args)]
pub struct RewindArgs {
    pub snapshot_id: String,
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct AttestArgs {
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub key: Option<PathBuf>,
    #[arg(long)]
    pub seed: Option<String>,
    #[arg(long)]
    pub slsa_l3: bool,
}

#[derive(Debug, Args)]
pub struct VerifyArgs {
    pub attestation_file: PathBuf,
    pub path: Option<PathBuf>,
    #[arg(long)]
    pub public_key: Option<String>,
    #[arg(long)]
    pub trusted_keys: Option<PathBuf>,
    #[arg(long)]
    pub slsa_l3: bool,
}

/// Arguments for watch command
#[derive(Debug, Args)]
pub struct WatchArgs {
    #[command(flatten)]
    pub common: CommonArgs,
    #[arg(long, default_value = "build")]
    pub mode: watch::WatchAction,
    #[arg(long, default_value = "200")]
    pub debounce: u64,
    #[arg(long)]
    pub clear: bool,
    #[arg(long, hide = true)]
    pub once: bool,
    #[arg(long)]
    pub predictive: bool,
}

#[derive(Debug, Args)]
pub struct CleanArgs {
    pub path: Option<PathBuf>,
    #[arg(long, short = 'a')]
    pub all: bool,
}

/// Arguments for run command
#[derive(Debug, Args)]
pub struct RunArgs {
    pub path: Option<PathBuf>,
    #[arg(short = 'p', long)]
    pub package: Option<String>,
    #[arg(long)]
    pub bin: Option<String>,
    #[arg(short = 'j', long = "jobs")]
    pub jobs: Option<usize>,
    #[arg(short = 'v', long)]
    pub verbose: bool,
    #[arg(last = true)]
    pub args: Vec<String>,
}

/// Arguments for graph command
#[derive(Debug, Args)]
pub struct GraphArgs {
    pub path: Option<PathBuf>,
    #[arg(long, default_value_t = GraphFormat::Tree, value_enum)]
    pub format: GraphFormat,
    /// Disable automatic cross-language dependency inference so the rendered
    /// graph matches a `--no-infer-deps` build.
    #[arg(long = "no-infer-deps")]
    pub no_infer_deps: bool,
}

/// Arguments for cache server
#[derive(Debug, Args)]
pub struct CacheServerArgs {
    #[arg(long, default_value = "127.0.0.1:9091")]
    pub listen: String,
    #[arg(long)]
    pub auth_token: Option<String>,
    #[arg(long)]
    pub dir: Option<PathBuf>,
}

/// Arguments for worker
#[derive(Debug, Args)]
pub struct WorkerArgs {
    #[arg(long, default_value = "127.0.0.1:9092")]
    pub listen: String,
    #[arg(long)]
    pub auth_token: Option<String>,
    #[arg(long, default_value = "fish-worker-node")]
    pub name: String,
    #[arg(long, default_value = "8")]
    pub max_concurrency: usize,
}

/// Arguments for plugin command
#[derive(Debug, Args)]
pub struct PluginArgs {
    /// Project directory
    #[arg(long, short)]
    pub path: Option<PathBuf>,
    #[command(subcommand)]
    pub action: PluginAction,
}

/// Plugin subcommands
#[derive(Debug, Subcommand)]
pub enum PluginAction {
    List,
    Audit {
        path: PathBuf,
    },
    Search {
        query: Option<String>,
        #[arg(long)]
        registry: Option<String>,
    },
    Install {
        name: String,
        #[arg(long)]
        version: Option<String>,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        trusted_key: Option<String>,
        #[arg(long)]
        insecure: bool,
    },
    Uninstall {
        name: String,
    },
    Publish {
        wasm_path: PathBuf,
        #[arg(long)]
        name: String,
        #[arg(long)]
        version: String,
        #[arg(long)]
        url: String,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        seed: Option<String>,
    },
    Execute {
        name: String,
        command: String,
        #[arg(allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

/// Graph output format
#[derive(Debug, Clone, clap::ValueEnum, Default)]
pub enum GraphFormat {
    #[default]
    Tree,
    Json,
    Dot,
}

#[derive(Debug, Args)]
pub struct AiArgs {
    #[command(subcommand)]
    pub action: AiAction,
}

#[derive(Debug, Subcommand)]
pub enum AiAction {
    Analyze {
        #[arg(long, short, default_value = "rust")]
        toolchain: String,
        #[arg(long)]
        stderr: Option<String>,
        #[arg(long)]
        file: Option<PathBuf>,
        #[arg(long, default_value_t = 1)]
        exit_code: i32,
    },
    Optimize {
        #[arg(long, short)]
        path: Option<PathBuf>,
        #[arg(long, default_value_t = 8)]
        workers: usize,
    },
    Recommend {
        #[arg(long, short)]
        path: Option<PathBuf>,
        #[arg(long)]
        files: Vec<String>,
    },
    Ping,
}

#[derive(Debug, Args)]
pub struct WhyArgs {
    pub target: String,
    #[arg(long, short)]
    pub path: Option<PathBuf>,
}

#[derive(Debug, Args)]
pub struct PashArgs {
    pub file: PathBuf,
    #[arg(long, short)]
    pub compare_with: Option<PathBuf>,
    #[arg(long, short)]
    pub lang: Option<String>,
    #[arg(long)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct QpcArgs {
    #[command(subcommand)]
    pub command: QpcCommand,
}

#[derive(Debug, Subcommand)]
pub enum QpcCommand {
    Status,
    Bench,
    Morphic {
        #[arg(long, short)]
        path: Option<PathBuf>,
    },
}
