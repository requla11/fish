# Changelog

> 🌐 **Translations & Contributions:** Want to translate or improve this document in your language? See our [Translation Guidelines](TRANSLATION.md).

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Cloud Cost Calculator (`fish cost-estimate`) — TOML pricing catalogs for AWS/GCP/Azure with LPT bin-packing, spot/ondemand comparison, egress/storage pricing, and ranked savings reports.
- OpenTelemetry OTLP/HTTP+JSON exporter (`OtlpExporter`) honoring `OTEL_EXPORTER_OTLP_ENDPOINT`; `fish build` exports root + per-task spans at completion.
- Distributed Trace Aggregation (`merge_worker_traces`) — dedup, trace-id adoption, orphan re-parenting across workers.
- Build Regression Alerts — median-baseline evaluation over rolling JSONL history, surfaced by `fish build`.
- Spot Instance Optimization (`PreemptionRetryExecutor`) — retries infrastructure-shaped failures then migrates to an on-demand fallback.
- Plugin Capability Auditor — risk-ranked static analysis of wasm plugin manifests.
- Live OSV advisory feed (`OsvClient`, `FISH_OSV_ENDPOINT`) — batched querybatch lookups replacing the stale embedded snapshot for Cargo and npm.
- In-toto Statement/v1 SLSA provenance model with Ed25519-signed statements and subject-binding verification.
- RBAC resource-scoped target rules (e.g. `prod/*` requiring higher clearance) and append-only JSONL audit log.
- CAS synchronous reader (`with_artifact_bytes`) for hashing/streaming without async overhead.

### Fixed
- Remote CAS downloads now verify BLAKE3 integrity; `stats()`/`list()` propagate errors instead of fabricating zeros.
- CAS GC rewritten for the real sharded layout; chunk reconstruction verifies per-chunk hashes and lengths.
- Windows LLD linking uses `-C linker=lld-link` (the previous flag broke link.exe).
- Response files follow MSVC backslash/quote escaping rules and reject embedded newlines.
- Work-stealing scheduler records real start offsets and worker ids in Chrome traces.
- Racing executor checks cancellation before starting each side and documents duplicated-execution semantics.
- fs watcher sets its running flag only after a successful start and honors the configured debounce interval.
- Jobserver pool clamps limit to ≥ 1; resource governor warning threshold no longer saturates at low limits.

### Removed
- WASM plugin engine no longer fakes hook execution or substitutes stub bytecode (fails loudly with `Unsupported` until a runtime is embedded).
- `fish fix` parses real `cargo check --message-format=json` diagnostics instead of printing fabricated ones.
- Time-machine stores artifact blobs under BLAKE3 digests and verifies them before rewind (no more placeholder content).
- Daemon status reports RUNNING instead of an untracked WARMED state.
- Docker builder extracts real image ids from build output rather than hashing stdout into a synthetic id.
- Incremental analyzer drops invented percentage claims from suggestions.
- eBPF tracer renamed to `FileEventRecorder` to honestly reflect its manual-feed nature.
- Micro-JIT assembler validates operands so disassembly matches emitted bytes; refuses unsupported register pairs and AArch64.
- Hot-patch delta computation refuses differing binaries instead of inventing synthetic relocations.
- Super-optimizer CFG recovery refuses instead of producing fake basic blocks.
- Python AI semantic_impact/prewarmer return honest statuses derived from path conventions instead of invented targets.

### Changed
- CI: repaired hallucinated action versions across all 22 workflows, defragmented triggers (5 lean PR pipelines), added polyglot gate with Python/Go/MSRV/macOS/cargo-deny jobs, Swatinem/rust-cache, and `--locked` cargo commands.
- GitLab CI: added fmt/clippy lint stages with shared Cargo.lock-keyed cache; fixed release artifact path.
- CONTRIBUTING.md: added provenance and human-review rules to the AI-Assisted Development Policy.

### Security
- Signature verification reports `metadata_verified=false` unless the SBOM matches explicit expectations (new `verify_artifact_with_metadata` API).

### Added (previously unreleased)
- Compilation database generator (`CompilationDatabase`, `compile_commands.json`) for Clangd and IDE integration.
- Hermetic toolchain manager (`ToolchainRegistry`, `ToolchainSpec`) for isolated toolchain environments.
- Real-time filesystem watcher daemon (`FsWatcherDaemon`) with hot node cache pre-warming.
- Interactive SVG DAG visualizer and 5-language localized Web UI (English, Vietnamese, Simplified Chinese, Traditional Chinese, Japanese).
- Copy-on-write and hardlink cloner (`KernelCowCloner`) for fast artifact materialization.
- Fast linker dispatcher (`LinkerDispatcher`) supporting auto-detection of `mold`, `lld`, and `msvc`.
- Kernel resource governor (`KernelResourceGovernor`) for memory pressure detection and concurrency control.
- Compiler pipelining coordinator (`PipelinedCompilationCoordinator`) unblocking downstream packages upon metadata readiness.
- Dynamic graph expander (`DynamicGraphExpander`) for runtime DAG expansion during task execution.
- Micro-input globbing filter (`MicroInputFilter`) for fine-grained input tracking.
- Algebraic graph query engine (`Fish query` with `deps()`, `rdeps()`, `allpaths()`, `somepath()`, `filter()`).
- Automated AST dependency inference (`DependencyInferenceEngine`) for Rust, TypeScript/JavaScript, Python, and Go.
- Distributed Task Execution (DTE) bin-packing (`DteBinPacker`) for multi-agent CI balancing.
- Dirty rebuild diagnostics (`DirtyExplainer`, `fish build --explain`).
- GNU Jobserver pool integration (`JobserverPool`) for compiler thread token coordination.
- Dynamic remote racing (`DynamicRacingExecutor`) racing local and remote worker execution.
- Background build daemon (`FishDaemon`, `Fish daemon start/status/stop`).
- Response file synthesis (`ResponseFileWriter`, `@fish_args.rsp`) for compiler argument vectors.
- Profile-Guided Optimization (PGO) orchestration (`PgoManager`, `--pgo-generate`, `--pgo-use`).
- Task pipeline topology engine (`PipelineResolver`) with cross-package `^build` dependencies.
- Stage Tree DAG visualizer for terminal rendering.
- Support for 11+ language backends (Rust, C/C++, Go, TypeScript, Python, Java, .NET, Swift, Dart, Zig, Docker)
- CAS (Content-Addressable Storage) artifact cache with Blake3 and Zstandard compression
- Distributed cluster execution with remote workers and binary streaming protocol
- Virtual File System (VFS) for on-demand file streaming
- Plugin system with ScriptPlugin support (Shell, Python, Node, WASM, Lua)
- CI/CD generator for GitHub Actions, GitLab CI, CircleCI, and Bitbucket Pipelines
- Build artifact signing & verification with Ed25519 and SPDX SBOM generation
- Dependency vulnerability scanner and HashiCorp Vault / AWS Secrets Manager integration
- Build cache analytics dashboard and flaky test detection & auto-retry

### Changed
- Updated repository to Foursavage organization (foursavage-dev)
- Raised MSRV from 1.86 to 1.88

### Dependencies
- Updated ratatui 0.30.0 -> 0.30.2, ratatui-crossterm 0.1.0 -> 0.1.2, ratatui-widgets 0.3.0 -> 0.3.2
- Updated time 0.3.45 -> 0.3.55 (time-core 0.1.9, time-macros 0.2.32)
- Updated ICU provider crates 2.2.0 -> 2.3.0
- Updated num-conv 0.1.0 -> 0.2.2, instability 0.3.10 -> 0.3.13, cargo-platform 0.3.1 -> 0.3.2
- Added darling 0.24.0 for derive macros; removed strum 0.27.2 / strum_macros 0.27.2
- Resolved RUSTSEC-2026-0253 via lru 0.18.2 / ratatui-core 0.1.2

### Security
- Artifact signing with cryptographic signatures
- Dependency vulnerability scanning
- Secure secret management
- Capability-based VFS permissions
- Resolved RUSTSEC-2026-0253 (lru use-after-free, fixed in lru 0.18.2)

## [0.1.0] - 2026-08-15

### Added
- Initial public release
- Core build orchestration engine
- Cache-first incremental builds
- Multi-language backend support
- Basic CI/CD generation

[Unreleased]: https://github.com/requla11/fish/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/requla11/fish/releases/tag/v0.1.0
