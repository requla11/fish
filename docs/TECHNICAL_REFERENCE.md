# Fish Build Orchestration System - Technical Reference

> **Version**: 0.6.0
> **Last Updated**: 2026-08-22
> **Rust Edition**: 2024
> **MSRV**: 1.88+

---

## Executive Summary

Fish is a high-performance, cache-first build orchestration system designed for polyglot monorepos. Written entirely in Rust 2024, it provides sub-millisecond task scheduling, automatic cross-language dependency inference, and content-addressable artifact storage with BLAKE3 hashing and ZSTD compression.

### Key Metrics

- **Workspace Size**: 28 crates, ~50,000+ lines of Rust code
- **Language Support**: 11 native backends (Rust, Go, TypeScript, Python, C/C++, Java, .NET, Swift, Dart, Zig, Docker)
- **Scheduling Overhead**: <100µs per task dispatch
- **Cache Hit Rate**: Up to 95%+ on incremental builds
- **Concurrency**: Lock-free work-stealing with Chase-Lev queues

### Architecture Philosophy

1. **Cache-First**: Every build operation is cacheable; cache hits are the default, not the exception
2. **Polyglot Native**: No DSLs required - uses existing toolchains (Cargo, npm, go build, etc.)
3. **Hermetic**: Isolated execution environments with deterministic outputs
4. **Zero-Config**: Automatic workspace discovery and dependency inference
5. **Distributed Ready**: Native support for remote workers and tiered caching

---

## Architecture Overview

### System Boundaries

```
┌─────────────────────────────────────────────────────────────────┐
│                         Fish CLI                                 │
│                    (User Entry Point)                             │
└───────────────────────────┬─────────────────────────────────────┘
                            │
                            │ JSON-RPC / IPC
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Core Orchestrator                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │ fish-core    │  │ fish-graph   │  │ fish-scheduler│         │
│  │ Discovery    │  │ DAG Engine   │  │ Work-Stealing │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
└───────────────────────────┬─────────────────────────────────────┘
                            │
            ┌───────────────┼───────────────┐
            │               │               │
            ▼               ▼               ▼
┌──────────────────┐ ┌──────────────┐ ┌──────────────┐
│ fish-executor    │ │ fish-cache   │ │ fish-cas     │
│ Process Spawn    │ │ Fingerprint  │ │ CAS Storage  │
│ File I/O         │ │ Invalidation │ │ Compression  │
└──────────────────┘ └──────────────┘ └──────────────┘
            │               │               │
            └───────────────┼───────────────┘
                            │
            ┌───────────────┼───────────────┐
            │               │               │
            ▼               ▼               ▼
┌──────────────────┐ ┌──────────────┐ ┌──────────────┐
│ Language Backends│ │ fish-worker  │ │ fish-security│
│ (11 ecosystems)  │ │ Distributed  │ │ Vulnerability │
│                  │ │ Execution    │ │ Scanning     │
└──────────────────┘ └──────────────┘ └──────────────┘
```

### Component Responsibilities

| Component | Responsibility | Key Abstractions |
|-----------|---------------|-------------------|
| **fish-core** | Workspace discovery, manifest parsing, toolchain management | `Project`, `ToolchainRegistry`, `ZeroConfigAdapter` |
| **fish-graph** | Dependency graph construction, topological sorting, query algebra | `BuildGraph`, `GraphNode`, `GraphQueryEngine` |
| **fish-executor** | Process execution, middleware chain, response file generation | `TaskExecutor`, `CommandSpec`, `TaskMiddleware` |
| **fish-scheduler** | Parallel task distribution, work-stealing, resource governance | `WorkStealingScheduler`, `JobserverPool`, `ResourceGovernor` |
| **fish-cache** | Fingerprint computation, cache invalidation, morphic hashing | `LocalCache`, `MorphicFingerprintEngine`, `FingerprintRecord` |
| **fish-cas** | Content-addressable storage, compression, deduplication | `CasStorage`, `Artifact`, `ChunkManifest` |
| **fish-security** | Vulnerability scanning, artifact signing, policy enforcement | `VulnerabilityScanner`, `OSVClient`, `SignedArtifactGate` |

---

## Core Components Deep Dive

### 1. fish-core - Workspace Discovery & Manifest Management

**Location**: `crates/fish-core/src/`

**Purpose**: Discover project structure, parse manifests, and manage toolchain configuration.

#### Key Modules

- **`adapters.rs`**: Zero-config monorepo detection
  - Automatically detects workspace type (Cargo, pnpm, Go work, Maven, etc.)
  - Returns `MonorepoDiscoveryResult` with suggested backend
  - Supports 8 monorepo types out of the box

- **`project/model.rs`**: Cargo workspace modeling
  - Wraps `cargo_metadata` for Rust workspaces
  - Provides `packages_for_paths()` for affected package detection
  - Computes build order via topological sort

- **`toolchain.rs`**: Hermetic toolchain management
  - `ToolchainRegistry` manages available toolchains
  - `ToolchainSpec` defines versioned toolchain requirements
  - Cross-platform executable discovery

- **`config.rs`**: Global configuration parsing
  - `FishConfig` from `fish.toml`
  - Support for build, cache, security, and CI configurations
  - Validation and error reporting

#### Design Decisions

**Why Zero-Config Discovery?**
- Reduces onboarding friction - no `fish.config` required
- Matches developer expectations (like Turborepo's auto-detection)
- Falls back gracefully to manual configuration

**Why Cargo Metadata for Rust?**
- Leverages battle-tested cargo tooling
- Provides accurate dependency graphs
- Supports workspaces and path dependencies natively

---

### 2. fish-graph - Dependency Graph Engine

**Location**: `crates/fish-graph/src/`

**Purpose**: Model build dependencies, compute execution order, and support graph queries.

#### Core Data Structures

```rust
pub struct BuildGraph<T> {
    nodes: Vec<GraphNode<T>>,
    edges: Vec<Vec<NodeId>>,
    reverse_edges: Vec<Vec<NodeId>>,
    states: Vec<TaskState>,
}
```

#### Key Operations

- **Topological Sort**: Kahn's algorithm for linear execution order
- **Ready Nodes**: Identify tasks with all dependencies satisfied
- **Dependency Tracking**: Bidirectional edge tracking for fast lookups
- **Query Algebra**: `deps()`, `rdeps()`, `somepath()`, `allpaths()`

#### Performance Characteristics

- **Graph Construction**: O(V + E) where V = nodes, E = edges
- **Topological Sort**: O(V + E)
- **Ready Node Query**: O(1) per node with precomputed indegree
- **Graph Merging**: O(V + E) for subgraph combination

#### Design Decisions

**Why Adjacency Lists?**
- Sparse graph representation (most tasks have few dependencies)
- Fast iteration over neighbors
- Memory-efficient for large graphs

**Why Type-Polymorphic Graph?**
- Supports different task payloads (build tasks, test tasks, etc.)
- Enables graph reuse across backends
- Maintains separation of concerns

---

### 3. fish-scheduler - Work-Stealing Scheduler

**Location**: `crates/fish-scheduler/src/`

**Purpose**: Distribute tasks across workers with minimal overhead and maximum parallelism.

#### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    WorkStealingScheduler                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Worker 1     │  │ Worker 2     │  │ Worker N     │      │
│  │ Local Queue  │  │ Local Queue  │  │ Local Queue  │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
│         │                  │                  │             │
│         └──────────────────┼──────────────────┘             │
│                            │                                 │
│                    Chase-Lev Queue                           │
│                    (Lock-Free Stealing)                      │
└─────────────────────────────────────────────────────────────┘
```

#### Key Components

- **`work_stealing.rs`**: Lock-free task distribution
  - Chase-Lev deque for work stealing
  - Priority-based task selection (critical path first)
  - Historical duration tracking for better scheduling

- **`jobserver_pool.rs`**: GNU Jobserver integration
  - Token-based concurrency control
  - Cross-compiler coordination (Cargo, cc, etc.)
  - Prevents CPU oversubscription

- **`resource_governor.rs`**: System resource monitoring
  - Memory pressure detection
  - Dynamic concurrency adjustment
  - Prevents OOM on large builds

#### Scheduling Algorithm

1. **Critical Path Priority**: Tasks on longest path get highest priority
2. **Historical Weighting**: Use past execution time for load balancing
3. **Work Stealing**: Idle workers steal from busiest queues
4. **Backpressure**: Throttle when system resources are constrained

#### Performance Targets

- **Task Dispatch**: <100µs per decision
- **Queue Operations**: O(1) amortized
- **Stealing Overhead**: <1µs per steal attempt
- **Scalability**: Linear up to 64 workers

---

### 4. fish-cache - Fingerprint Cache

**Location**: `crates/fish-cache/src/`

**Purpose**: Compute and cache content fingerprints for incremental builds.

#### Fingerprint Strategy

**Exact Fingerprint**: BLAKE3 hash of file contents, environment variables, and compiler flags

**Morphic Fingerprint**: Normalized version that accounts for benign changes:
- Path normalization (relative to workspace root)
- Environment variable whitelisting
- Source code comment stripping
- Timestamp normalization

#### Key Modules

- **`morphic.rs`**: Morphic fingerprint engine
  - `MorphicEnvironmentFilter`: Whitelisted env vars only
  - `MorphicPathNormalizer`: Workspace-relative paths
  - `MorphicSourceNormalizer`: Comment/stripping

- **`strategies.rs`**: Cache invalidation strategies
  - Two-phase pruning (L1 memory + L2 disk)
  - TTL-based expiration
  - Size-based eviction

- **`pool.rs`**: Memory pooling for performance
  - `BufferPool`: Reusable byte buffers
  - `StringPool`: String interning
  - Reduces allocation overhead

#### Cache Layout

```
~/.fish/cache/
├── metadata/          # Fingerprint records
│   ├── <hash>/        # Sharded by hash prefix
│   └── ...
├── objects/           # Content-addressable objects
│   ├── <blake3-hash>  # Deduplicated by content
│   └── ...
└── artifacts/         # Build outputs
    └── ...
```

#### Performance Optimizations

- **Memory Cache**: DashMap for concurrent lookups
- **Disk Stats Cache**: Cached metadata to avoid syscalls
- **Atomic Writes**: Write-then-rename for crash safety
- **Buffer Pooling**: Reuse buffers across operations

---

### 5. fish-cas - Content-Addressable Storage

**Location**: `crates/fish-cas/src/`

**Purpose**: Store build artifacts by content hash with compression and deduplication.

#### Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    CasStorage                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │ Compression  │  │ Chunking     │  │ Deduplication│      │
│  │ ZSTD Level 3 │  │ FastCDC      │  │ BLAKE3       │      │
│  └──────────────┘  └──────────────┘  └──────────────┘      │
└───────────────────────────┬─────────────────────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
        ▼                   ▼                   ▼
┌──────────────┐   ┌──────────────┐   ┌──────────────┐
│ Local Backend│   │ Remote Backend│  │ mmap Support  │
│ File System  │   │ S3/GCS/MinIO │  │ Zero-Copy     │
└──────────────┘   └──────────────┘   └──────────────┘
```

#### Key Features

- **BLAKE3 Hashing**: Fast cryptographic hashing (AVX2/AVX-512 optimized)
- **ZSTD Compression**: Level 3 default (balance speed/ratio)
- **FastCDC Chunking**: Content-defined chunking for deduplication
- **Zero-Copy Reads**: Memory-mapped files for hot artifacts
- **io_uring Support**: Linux async I/O for high throughput

#### Storage Format

```
Artifact Header:
- Hash: 32 bytes (BLAKE3)
- Size: 8 bytes
- Compression: 1 byte
- Flags: 1 byte

Chunks:
- Chunk Hash: 32 bytes
- Chunk Size: 4 bytes
- Chunk Data: variable
```

#### Performance Metrics

- **Hashing Speed**: ~1GB/s on modern CPUs
- **Compression Ratio**: 3-5x typical (10x for binaries)
- **Deduplication**: 30-50% space savings on similar artifacts
- **Read Speed**: 500MB/s (mmap), 200MB/s (compressed)

---

### 6. fish-security - Vulnerability Scanning & Artifact Signing

**Location**: `crates/fish-security/src/`

**Purpose**: Scan dependencies for vulnerabilities and sign build artifacts.

#### Vulnerability Scanning

**Sources**:
- **Embedded Database**: Offline-friendly snapshot (ages quickly)
- **OSV API**: Live lookups against `api.osv.dev` (configurable endpoint)

**Supported Ecosystems**:
- Rust (via `cargo-audit` / OSV)
- npm/Node.js (via OSV)
- Maven/Java (embedded rules)

**Severity Levels**:
- Critical, High, Medium, Low, None
- Configurable blocking thresholds
- CVSS score tracking

#### Artifact Signing

**Algorithm**: Ed25519 (high-speed, small signatures)

**Workflow**:
1. Build artifact → Compute BLAKE3 hash
2. Sign hash with `FISH_SIGNING_SEED`
3. Store signature alongside artifact
4. Verify on retrieval with `FISH_TRUSTED_KEYS`

**SLSA Provenance**:
- Generates in-toto provenance statements
- Tracks source, build parameters, and materials
- Supports SPDX and CycloneDX SBOMs

#### Security Policy

```rust
pub enum SecurityLevel {
    Strict,      // Fail-closed, explicit allow-lists only
    Paranoid,    // All checks, network isolation
    AllowAll,    // No restrictions (development only)
}
```

---

## Language Backends

### Backend Interface

All backends implement the `EcosystemBackend` trait:

```rust
pub trait EcosystemBackend: Send + Sync {
    fn id(&self) -> &'static str;
    fn ecosystems(&self) -> &'static [Ecosystem];
    fn detect(&self, dir: &Path) -> bool;
    fn build_task_graph(
        &self,
        dir: &Path,
        mode: BuildMode,
    ) -> Result<BuildGraph<Task>, String>;
}
```

### Supported Backends

| Backend | Manifest | Default Tasks | Special Features |
|---------|----------|---------------|-----------------|
| **Rust** | `Cargo.toml` | check, build, test | Pipelined compilation, PGO support |
| **Go** | `go.mod` | vet, build, test | Go work support |
| **TypeScript** | `package.json` | typecheck, build, test | tsconfig detection |
| **Python** | `pyproject.toml` | syntax compile, pytest, lint | venv management |
| **C/C++** | `CMakeLists.txt` | configure, build, ctest | depfile parsing |
| **Java** | `pom.xml`, `build.gradle` | compile, test | Maven/Gradle support |
| **.NET** | `*.csproj`, `*.sln` | build, test | Multi-target frameworks |
| **Swift** | `Package.swift` | build, test | SPM integration |
| **Dart** | `pubspec.yaml` | analyze, test | Flutter support |
| **Zig** | `build.zig` | build, test | Zig std integration |
| **Docker** | `Dockerfile` | multi-stage build | Layer caching |

### Adding a New Backend

1. Create `crates/fish-backend-{lang}/`
2. Implement `EcosystemBackend` trait
3. Add to workspace members in `Cargo.toml`
4. Register in `fish-cli/src/backend_registry.rs`
5. Extend `Ecosystem` enum in `fish-backend-api`

---

## Data Flow

### Build Execution Flow

```
1. Workspace Discovery (fish-core)
   ↓
   Scan for manifests
   Detect monorepo type
   Load toolchain registry

2. Dependency Graph Construction (fish-graph)
   ↓
   Parse manifest files
   Extract package dependencies
   Build DAG of tasks

3. Fingerprint Computation (fish-cache)
   ↓
   Hash source files
   Compute environment fingerprint
   Check cache for hits

4. Task Scheduling (fish-scheduler)
   ↓
   Identify ready tasks
   Distribute to workers
   Manage concurrency

5. Task Execution (fish-executor)
   ↓
   Spawn compiler processes
   Capture stdout/stderr
   Handle timeouts

6. Artifact Storage (fish-cas)
   ↓
   Compute BLAKE3 hash
   Compress with ZSTD
   Store in CAS

7. Cache Population (fish-cache)
   ↓
   Store fingerprint record
   Link to artifact hash
   Update statistics
```

### Distributed Build Flow

```
1. Worker Registration
   ↓
   Workers connect to coordinator
   Report capabilities (CPU, RAM, toolchains)

2. Task Distribution
   ↓
   Coordinator partitions tasks
   Assigns to workers based on affinity
   Streaming VFS for inputs

3. Remote Execution
   ↓
   Workers execute in isolation
   Stream results back
   Handle failures/retries

4. Result Aggregation
   ↓
   Collect outputs from workers
   Verify artifacts
   Populate remote cache
```

---

## Performance Characteristics

### Bottlenecks & Optimizations

| Component | Bottleneck | Optimization | Impact |
|-----------|-----------|-------------|--------|
| **Scheduling** | Graph traversal | Precomputed tail lengths | 10x faster |
| **Cache** | Disk I/O | Memory cache + buffer pooling | 5x faster |
| **CAS** | Compression | ZSTD level 3 + chunking | 3x faster storage |
| **Executor** | Process spawn | Response files for long commands | 2x faster |
| **Network** | Latency | P2P mesh + local caching | 50% reduction |

### Benchmark Targets

- **Cold Build**: Full rebuild, no cache
- **Warm Build**: 95%+ cache hit rate
- **Incremental**: Single file change, <5s
- **Graph Size**: 10,000 nodes, <1s scheduling
- **Throughput**: 1000 tasks/second per worker

### Profiling Tools

```bash
# CPU profiling
cargo flamegraph --bin fish

# Memory profiling
valgrind --leak-check=full fish build

# Build time profiling
fish build --profile build-trace.json
```

---

## Security Model

### Threat Model

1. **Malicious Dependencies**: Scanned via OSV
2. **Artifact Tampering**: Ed25519 signing
3. **Code Injection**: Hermetic sandboxing
4. **Privilege Escalation**: Least privilege execution
5. **Cache Poisoning**: Signature verification

### Security Controls

- **Input Validation**: All external inputs validated
- **Least Privilege**: Workers run with minimal permissions
- **Audit Logging**: All security operations logged
- **Secret Management**: Integration with Vault/AWS Secrets
- **Network Isolation**: Optional sandbox network policies

### Compliance

- **SLSA Level 3**: Provenance generation
- **SPDX/CycloneDX**: SBOM support
- **OSV**: Vulnerability database integration
- **Ed25519**: Cryptographic signing

---

## Development Guide

### Building

```bash
# Development build
cargo build -p fish-cli

# Release build
cargo build -p fish-cli --release

# Full workspace
cargo build --workspace
```

### Testing

```bash
# Unit tests
cargo test --workspace

# Integration tests
cargo test --test integration

# Specific crate
cargo test -p fish-core
```

### Code Quality

```bash
# Format
cargo fmt --all -- --check

# Lint
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Security audit
cargo audit
```

### Adding Features

1. **Identify Crate**: Choose appropriate crate for functionality
2. **Write Tests**: TDD approach, test first
3. **Implement**: Follow existing patterns
4. **Document**: Add rustdoc comments
5. **Verify**: Run full test suite

---

## Deployment Architecture

### Single Machine

```
Fish CLI → Local Cache → Local CAS → Compilers
```

### Distributed Team

```
Fish CLI → Local Cache → Remote Cache (S3/GCS) → Teammates
```

### Enterprise Scale

```
Fish CLI → Local Cache → Remote Cache → Worker Cluster
                ↓                    ↓
            Coordinator ←→ K8s Operator
                ↓
            Analytics Dashboard
```

### Monitoring

- **OpenTelemetry**: Distributed tracing
- **Metrics**: Cache hit rate, build duration, task throughput
- **Logs**: Structured JSON logs with tracing
- **Dashboard**: Web UI for real-time visualization

---

## Troubleshooting

### Common Issues

**Cache Misses Unexpectedly**
- Check fingerprint computation
- Verify environment variables
- Review morphic normalization rules

**Slow Scheduling**
- Profile graph construction
- Check for circular dependencies
- Verify worker count

**Memory Pressure**
- Reduce concurrency
- Enable resource governor
- Check for memory leaks

**Build Failures**
- Check toolchain versions
- Verify manifest syntax
- Review error diagnostics

### Debug Mode

```bash
# Enable debug logging
RUST_LOG=debug fish build

# Verbose output
fish build --verbose

# Cache diagnostics
fish doctor
```

---

## Future Roadmap

### v0.7.x - AI-Native Builds
- Compiler-grounded fix suggestions
- Natural-language build queries
- Learned resource governor
- Test selection model

### v0.8.x - Enterprise Features
- SSO integration
- RBAC policies
- Audit logging
- Compliance reporting

### v1.0 - Production Ready
- GA release
- SLA guarantees
- Enterprise support
- Managed service

---

## Appendix A: Glossary

- **BLAKE3**: Cryptographic hash function optimized for performance
- **CAS**: Content-Addressable Storage - store by content hash
- **DAG**: Directed Acyclic Graph - dependency graph without cycles
- **Morphic**: Normalized fingerprint that ignores benign changes
- **OSV**: Open Source Vulnerabilities database
- **PGO**: Profile-Guided Optimization
- **SLSA**: Supply-chain Levels for Software Artifacts
- **ZSTD**: Zstandard compression algorithm

---

## Appendix B: References

- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [BLAKE3 Specification](https://github.com/BLAKE3-team/BLAKE3-specs)
- [OSV API](https://osv.dev/docs/)
- [SLSA Specification](https://slsa.dev/)

---

**Document Maintained By**: Fish Core Team
**Feedback**: GitHub Issues - https://github.com/requla11/fish/issues
