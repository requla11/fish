# Fish Architecture

> 🌐 **Translations & Contributions:** Want to translate or improve this document in your language? See our [Translation Guidelines](TRANSLATION.md).

This document describes the high-level architecture of the fish build orchestration system.

## Overview

Fish is a cache-first, polyglot build orchestration system designed for monorepos and polyglot projects. It uses a dependency graph, parallel scheduler, executor, and CAS artifact cache to optimize build performance.

## Core Components

### 1. Workspace Discovery (`fish-core`)

**Purpose**: Discover and model the project structure

**Responsibilities**:
- Scan workspace for packages/projects
- Detect project types based on manifest files
- Filter input files by micro-globs (`MicroInputFilter`)
- Build dependency graph between packages
- Generate IDE compilation databases (`CompilationDatabase`, `compile_commands.json`)
- Manage and isolate hermetic compiler toolchains (`ToolchainRegistry`, `ToolchainSpec`)
- Manage package metadata

**Key Types**:
- `Package`: Represents a single package/project
- `Workspace`: Collection of packages with dependencies
- `Manifest`: Project configuration (Cargo.toml, package.json, etc.)
- `MicroInputFilter`: Fine-grained glob matcher and file filter
- `CompilationDatabase`: Standard compilation command database
- `ToolchainRegistry`: Hermetic toolchain configuration manager

### 2. Build Graph (`fish-graph`)

**Purpose**: Model build dependencies, execution order, and algebraic queries

**Responsibilities**:
- Create directed acyclic graph (DAG) of build tasks
- Compute topological sort for execution order
- Subgraph merging for polyglot monorepos (`merge_subgraph`)
- Dynamic node expansion during runtime execution (`DynamicGraphExpander`)
- Track task states (pending, running, completed, failed)
- Algebraic query evaluation (`GraphQueryEngine` supporting `deps()`, `rdeps()`, `allpaths()`, `somepath()`, `filter()`, `union()`, `intersect()`, `except()`)
- Detect circular dependencies

**Key Types**:
- `BuildGraph`: Directed acyclic graph of tasks
- `Node`: Individual build task
- `NodeId`: Type-safe index into graph structures
- `DynamicGraphExpander`: Dynamic sub-task generator
- `GraphQueryEngine`: Evaluator for graph query expressions
- `QueryExpr`: Algebraic query AST

### 3. Executor (`fish-executor`)

**Purpose**: Execute build commands, manage processes, and handle file system cloning

**Responsibilities**:
- Spawn and manage build processes
- Capture stdout/stderr
- Handle process timeouts and cancellation
- Fast file system cloning using copy-on-write extents and hardlinks (`KernelCowCloner`)
- Fast linker auto-detection and flag synthesis (`LinkerDispatcher` supporting `mold`, `lld`, and `msvc`)
- Automatic response file synthesis (`@fish_args.rsp`) when arguments exceed OS limits
- Extensible task middleware pipeline (`TaskMiddleware`, `TurboLinker`, `SuperOptimizer`)
- Return execution results

**Key Types**:
- `CommandSpec`: Command specification with environment
- `AsyncExecutor`: Non-blocking process execution engine
- `KernelCowCloner`: Copy-on-write and fast cloner
- `LinkerDispatcher`: Modern linker detector
- `ResponseFileWriter`: Argument file synthesizer
- `TaskMiddleware`: Middleware trait for task interception
- `ExecutionResult`: Result of command execution

### 4. Scheduler (`fish-scheduler`)

**Purpose**: Schedule tasks for parallel, speculative, and distributed execution

**Responsibilities**:
- Maintain ready queue of available tasks
- Distribute tasks across available workers
- Kernel resource governor (`KernelResourceGovernor`) monitoring system memory pressure and throttling concurrency
- Compiler pipelining coordination (`PipelinedCompilationCoordinator`) unblocking downstream compilation upon metadata readiness
- GNU Jobserver pool integration (`JobserverPool`) for global thread token management across compilers
- Dynamic remote racing (`DynamicRacingExecutor`): concurrent local vs remote execution
- Distributed Task Execution (DTE) bin-packing (`DteBinPacker`) using Longest Processing Time (LPT) scheduling
- Real-time filesystem watcher daemon (`FsWatcherDaemon`) with dirty node invalidation and hot graph cache pre-warming
- Respect task dependencies
- Handle task completion and failure

**Key Types**:
- `Scheduler`: Task scheduling engine
- `KernelResourceGovernor`: Memory pressure monitor
- `PipelinedCompilationCoordinator`: Pipelined stage manager
- `JobserverPool`: Global token-based concurrency pool
- `DynamicRacingExecutor`: Local vs remote racer
- `DteBinPacker`: Balanced multi-agent CI partitioner
- `FsWatcherDaemon`: Real-time change listener and dirty node tracker
- `WorkStealingPool`: Lock-free task distributor

### 5. Cache (`fish-cache`)

**Purpose**: Fingerprint-based caching for incremental builds

**Responsibilities**:
- Compute file content fingerprints (Blake3)
- Cache execution results
- Determine cache validity
- Support cache invalidation

**Key Types**:
- `Fingerprint`: Content hash with metadata
- `CacheEntry`: Cached execution result
- `FileLevelCache`: File-level caching strategy

### 6. CAS Engine (`fish-cas`)

**Purpose**: Content-Addressable Storage for artifact caching

**Responsibilities**:
- Store artifacts by content hash
- Support local and remote storage
- Compress artifacts (Zstandard)
- Provide deduplication

**Key Types**:
- `ArtifactStore`: Artifact storage interface
- `LocalStorage`: Local file system storage
- `RemoteStorage`: Remote storage (S3, GCS, MinIO)

### 7. Remote Cache (`fish-remote-cache`)

**Purpose**: Tiered L1/L2 composite caching

**Responsibilities**:
- Local L1 cache for fast access
- Remote L2 cache for sharing
- Cache population and eviction
- Cache hit/miss tracking

**Key Types**:
- `CompositeCache`: Tiered cache implementation
- `CachePolicy`: Cache population and eviction policies

### 8. Worker (`fish-worker`)

**Purpose**: Distributed build execution

**Responsibilities**:
- Remote worker discovery and registration
- Task distribution across workers
- Result collection and aggregation
- Virtual File System for on-demand file access

**Key Types**:
- `WorkerServer`: Worker daemon
- `ClusterExecutor`: Cluster task execution
- `VirtualFileSystem`: In-memory VFS

### 9. Sandboxing (`fish-sandbox`)

**Purpose**: Hermetic environment isolation

**Responsibilities**:
- Isolate build environments
- Control filesystem access
- Network isolation
- Resource limits

**Key Types**:
- `Sandbox`: Sandbox implementation
- `SandboxConfig`: Sandbox configuration

### 10. Plugin System (`fish-plugin`)

**Purpose**: Extensible rule system

**Responsibilities**:
- Load custom plugins
- Script plugin execution (Shell, Python, Node, WASM, Lua)
- Plugin discovery and management
- Plugin API

**Key Types**:
- `PluginManager`: Plugin manager
- `ScriptPlugin`: Script-based plugin
- `PluginExecutor`: Plugin execution engine

## Language Backends

Every backend implements the uniform `EcosystemBackend` contract from
the `fish-backend-api` crate and registers itself in
`fish-cli/src/backend_registry.rs` — adding an ecosystem means
implementing the trait plus one registry line.

### Backend Interface

```rust
pub trait EcosystemBackend: Send + Sync {
    fn id(&self) -> &'static str;
    fn ecosystems(&self) -> &'static [Ecosystem];
    /// Cheap existence probe for this ecosystem's manifests.
    fn detect(&self, dir: &Path) -> bool;
    /// Config discovery + task-graph construction, owned per backend.
    fn build_task_graph(
        &self,
        dir: &Path,
        mode: BuildMode,
    ) -> Result<BuildGraph<Task>, String>;
}
```

### Supported Backends

- **Rust** (`fish-backend-rust`): Cargo workspaces
- **C/C++** (`fish-backend-cc`): gcc/clang/msvc
- **Go** (`fish-backend-go`): go.mod
- **TypeScript/JS** (`fish-backend-ts`): package.json
- **Python** (`fish-backend-py`): pyproject.toml
- **Java** (`fish-backend-java`): Maven/Gradle
- **.NET** (`fish-backend-dotnet`): csproj/sln
- **Swift** (`fish-backend-swift`): Package.swift
- **Dart** (`fish-backend-dart`): pubspec.yaml
- **Zig** (`fish-backend-zig`): build.zig
- **Docker** (`fish-backend-docker`): Dockerfile

## Security Features

### 1. Artifact Signing (`fish-security` / `fish-remote-cache`)

- Ed25519 signing via `FISH_SIGNING_SEED`; public key exported with
  `fish signing-key`
- SLSA/in-toto provenance statements (`fish-security/src/slsa.rs`)
- Remote-cache signature gate verifies every download against
  `FISH_TRUSTED_KEYS` (`fish-remote-cache/signature_gate.rs`)
- See `docs/signing.md` for the full producer/consumer flow

### 2. Security Scanner (`fish-security`)

- Dependency vulnerability scanning
- Multi-backend support
- Severity-based blocking
- CVSS score tracking

## CI/CD Generation

### CI Generator (`fish-ci-generator`)

Supports multiple CI/CD platforms:
- GitHub Actions
- GitLab CI
- CircleCI
- Bitbucket Pipelines

### Matrix Generation

- Multi-platform support (Linux, macOS, Windows)
- Multi-architecture (x86_64, ARM64)
- Version matrices (Rust, Node, etc.)
- Dependency-based optimization

## Advanced Features

### 1. Build Analytics (`fish-analytics`)

- Real-time cache hit rate tracking
- Build metrics collection
- Performance visualization
- Optimization suggestions

### 2. Incremental Analysis (`fish-incremental`)

- AST-based dependency inference (`DependencyInferenceEngine`) for Rust, TypeScript/JavaScript, Python, and Go
- Dirty rebuild diagnostics (`DirtyExplainer`, `fish build --explain`) identifying exact source file modifications or hash mismatches
- Build pattern detection and hotspot identification
- Refactoring suggestions and rebuild frequency analysis

### 3. Build Daemon & IPC (`fish-cli::daemon`)

- Background daemon (`FishDaemon`) speaking JSON-RPC 2.0 over
  newline-delimited messages
- Transport: Unix domain socket on Unix, TCP on `127.0.0.1` on Windows
- Port is configurable: `fish daemon start --port <PORT>` (default `9527`)
- Warm graph caching for repeated invocations
- Commands: `fish daemon start`, `fish daemon status`, `fish daemon stop`

### 4. Profile-Guided Optimization (`fish-cli::pgo`)

- 2-phase LLVM PGO workflow orchestration (`PgoManager`)
- Automated `-Cprofile-generate` instrumentation and `llvm-profdata merge`
- Recompilation with `-Cprofile-use` for maximum runtime performance

### 5. Task Pipeline Topology (`fish-cli::pipeline`)

- Turborepo/Nx style topological task pipelines configured via `fish.toml`
- Cross-package dependency rules (e.g. `^build` ensuring dependency outputs are built first)
- Configurable environment variable and input file fingerprint hashes

### Planned crates (not yet in the workspace)

The following crates were described in earlier drafts but do not exist in
the workspace yet. They are listed here as roadmap items only:

- `fish-multiplatform` — platform detection, target triples, CI matrices
- `fish-notifications` — Slack/Discord/email build notifications
- `fish-flaky-detection` — statistical flaky test detection and retry policies
- `fish-docker-builder` — first-class Docker artifacts and layer caching
  (Docker orchestration today lives in `fish-backend-docker`)
- `fish-templates` — shareable pipeline templates (Handlebars rendering)

## Vendored submodules

Two companion projects are vendored as git submodules and are members of the
workspace:

- **`submodules/apple`** — hermetic sandbox and process isolation daemon
  (kernel-level sandboxing, CoW storage jails, SLSA/SPDX/CycloneDX
  provenance). Independent project, not affiliated with Apple Inc.
- **`submodules/banana`** — distribution, P2P swarm, and supply-chain
  infrastructure companion for Fish.

## Data Flow

### Build Execution Flow

```
1. Workspace Discovery
   ↓
2. Dependency Graph Construction
   ↓
3. Cache Fingerprint Computation
   ↓
4. Scheduler Task Distribution
   ↓
5. Executor Process Management
   ↓
6. Result Collection & Caching
   ↓
7. Build Completion
```

### Distributed Build Flow

```
1. Worker Registration
   ↓
2. Task Distribution
   ↓
3. VFS File Streaming
   ↓
4. Remote Execution
   ↓
5. Result Aggregation
   ↓
6. Cache Population
```

## Performance Optimizations

### 1. Level Partitioning

Groups independent packages per build level into single toolchain calls, eliminating process spawn overhead.

### 2. Cache-First Execution

Fingerprint-based caching enables instant rebuilds when inputs haven't changed.

### 3. Parallel Execution

Tasks are executed in parallel respecting dependencies, maximizing CPU utilization.

### 4. Incremental Builds

Only rebuild affected packages based on dependency graph changes.

### 5. Distributed Execution

Remote workers enable horizontal scaling for large projects.

## Architecture Status

Fish is a single-language Rust workspace. There are no Python or Go services
in this repository, and no crate currently uses gRPC/protobuf. Earlier drafts
of this document described a "Tri-Engine" architecture; that description did
not match the codebase and has been removed.

### Current core (implemented)

- **`fish-core`**: Workspace discovery, manifest models, fine-grained input filtering.
- **`fish-graph`**: Dependency graph, topological sort, algebraic query evaluation (`deps`, `rdeps`, `somepath`).
- **`fish-executor`**: Process execution, middleware chain, response file generation.
- **`fish-scheduler`**: GNU Jobserver pool, work-stealing, parallel execution.
- **`fish-cache`**: Multi-tier fingerprinting with Blake3 and two-phase pruning.
- **`fish-cas`**: Content-addressable artifact storage with ZSTD compression.
- **`fish-cli`**: Terminal user interface powered by ratatui and clap.

### Planned: cross-language contracts (`proto/`)

The files under `proto/fish/v1/` (`build.proto`, `ai.proto`,
`coordinator.proto`) are forward-looking interface drafts only. They are not
compiled or referenced by any crate yet — the workspace has no `prost`/`tonic`
dependencies. Distributed features shipped today use plain HTTP/JSON instead
(see `crates/fish-worker` and `crates/fish-remote-cache`).

## Security Considerations

- No unsafe code in security-sensitive crates
- Input validation across all backends
- Least privilege for all operations
- Audit logging for security operations
- Secure secret management
- Ed25519 artifact signing and cryptographic SBOM generation
