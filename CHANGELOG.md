# Changelog

> ðŸŒ **Translations & Contributions:** Want to translate or improve this document in your language? See our [Translation Guidelines](TRANSLATION.md).

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **Dependency cycles are now reported with their full path.** `BuildGraph::find_cycle()`
  returns the actual node sequence forming a cycle (deterministic DFS), `validate()` and
  `add_dependency()` embed that closed walk — e.g. `dependency cycle detected: 0 -> 1 ->
  2 -> 0` — instead of a placeholder `edge 0 -> 0` error or bare endpoint names, and the
  lockless critical-path computation reports the cycle segment it traversed rather than a
  single node.

### Changed
- **Backends declare their concrete build outputs as task artifacts** — go
  (output binary), cc (linked binary), zig (`zig-out/` tree), java maven
  (`target/`) and gradle (`build/libs/`), dotnet (`--output` dir when
  configured) and python pex (`.pex` file). Together with local artifact
  restore this completes the cache-first loop for every ecosystem, and
  cross-language inference can now link consumers to just the producing
  tasks instead of whole projects.
- **Local cache hits now restore declared task artifacts.** On success,
  `CachingExecutor` packs each task's declared outputs into content-addressed
  objects with a hashed manifest; on a fingerprint hit it re-materializes any
  missing file from that store and falls back to a real rebuild whenever the
  store is incomplete or the record predates artifact tracking. Previously a
  local cache hit reported success while leaving no outputs on disk unless a
  remote cache was configured.

### Fixed
- **cc**: object files, depfiles, and cache keys are discriminated by the
  source's project-relative path — same-stem sources in different directories
  no longer overwrite each other's objects or share one cache record.
- **rust**: package fingerprints no longer exclude every directory named
  `bin`; edits to the standard `src/bin/*` binary targets now invalidate the
  cache instead of serving stale binaries.
- **go**: build/test fingerprints include race, coverage, ldflags, gcflags,
  and env; flipping `-race` no longer replays another configuration's cached
  results. The documented `run_linter` knob is honored (vet skipped when
  disabled) and vet is cached like its sibling tasks.
- **zig / dart / swift / dotnet**: fingerprints now include release mode (and
  dart's target platform), so toggling configurations can no longer replay
  the other side's cached builds.
- **zig**: the default test task emits `zig build test` under build.zig
  projects and resolves a root source file otherwise; with neither present
  the task is omitted instead of scheduling a guaranteed failure.
- **dart**: `dart compile exe` resolves its entrypoint and `-o` output
  (declared as task artifacts) instead of always failing for plain-Dart
  projects.
- **python**: default lint/typecheck/test tasks gate on their tool being
  available on PATH, and the build step follows the detected runner
  (uv/poetry) instead of hardcoding uv.
- **manifest parsing**: project-name extraction became boundary-aware
  everywhere — gradle `namespace` no longer satisfies the `name` key
  (`rootProject.name` is preferred), pom coordinates are read after the
  `<parent>` block instead of the first matching tag, pubspec `hostname:`
  lookalikes no longer win over the top-level `name:`, and build.zig's
  `.name_hash` cannot satisfy `.name`.
- **toolchain detection**: zig/dart/dotnet/java share one cross-platform
  executable lookup instead of shelling out to Windows-only `where` and
  blessing installs whose version probe merely spawned.

- **java**: maven `package` always passes `-DskipTests` and gradle `build`
  passes `-x test`, so suites run exactly once through the dedicated cached
  test task; clean-task labels use artifact_id consistently.

### CI
- Heavyweight suites (fuzzing, mutation-testing, sanitizers,
  flaky-quarantine, performance-benchmarks, multi-platform,
  reproducible-builds, integration-testing, backend-testing, dogfood) are now
  manual-only (`workflow_dispatch`) — their cron schedules and main-push
  triggers fired expensive runs on a fresh repository before any baseline
  existed. `essential-ci` gains push/PR triggers on `main`+`dev` (it was
  manual-only), and security-audit keeps its weekly heartbeat.

## [0.6.0] - 2026-08-25

### Added
- `fish-backend-api`: the real `EcosystemBackend` trait (id/ecosystems/detect/build_task_graph) — every backend now implements one contract and registers in a single registry; adding an ecosystem is implement-trait + one line
- `fish heal`: git-bisects recent commits on build failure, prepares a local revert branch with ready-to-paste PR copy (never pushes)
- `fish init --describe "rust cli + python tools"`: rule-based natural-language scaffolding
- `fish gen-docs`: renders docs/cli-reference.md from clap definitions; CI fails on drift (`CLI Docs Drift` job)
- `fish signing-key`: exports the Ed25519 public key from FISH_SIGNING_SEED; docs/signing.md documents the full flow

### Changed
- Workspace slimmed from 35 to 27 crates: removed 8 dead crates (~3.1k lines) including fish-dashboard, fish-docker-builder, fish-secrets, fish-signing, fish-multiplatform, fish-notifications, fish-flaky-detection, fish-templates
- polyglot dispatcher is data-driven (was a 170-line per-ecosystem match); BuildMode moved to fish-backend-api
- Cross-language dependency inference for polyglot workspaces (--no-infer-deps to disable)

### Performance
- Polyglot builds and `fish graph` now perform a single full-tree ecosystem
  walk per invocation: command dispatch passes its scan result into the graph
  builder instead of walking the workspace twice.
- Tree walkers classify entries through `DirEntry::file_type()` (the type the
  OS already delivered) instead of two separate `is_dir()`/`is_file()` stat
  calls per entry.
- Cross-language inference resolves references lexically before touching the
  filesystem, eliminating up to hundreds of thousands of pointless `exists()`
  stats on import-heavy repositories, and no longer allocates for string
  literals that cannot escape their project.
- Discovery and cross-project scanning share one prune list, so vendored and
  dependency trees are skipped consistently by every pass.

### Added
- Cross-language dependency inference for polyglot workspaces: fish scans each
  detected project for references into sibling projects (source imports that
  reach across directories, `go.mod` `replace` pointers, `-e ../` editable
  requirements) and links the corresponding tasks so producers build first â€”
  no `depends_on` declarations needed. On by default; disable with
  `--no-infer-deps`. Every edge cites its evidence file in build logs, and
  mutual references are refused instead of guessed. `fish graph` now renders
  the unified task graph (inferred edges included) for multi-ecosystem
  workspaces instead of walking up to an enclosing Cargo workspace.

### Fixed
- Docker backend: a Dockerfile using lowercase `as` stage aliases had every
  instruction collapse into one "default" bucket, emitting N identically-named
  tasks that ran as N redundant concurrent full-image builds. Stage parsing is
  now case-insensitive, unnamed `FROM` stages get stable names derived from
  their image reference, exactly one task is emitted per stage, and the image
  artifact attaches to the actual last stage instead of the magic name
  `final`.

## [0.5.0] - 2026-08-24

### Added
#### Mid-term (v0.4â€“v0.5)
- Cloud Cost Calculator (`fish cost-estimate`) â€” TOML pricing catalogs for AWS/GCP/Azure with LPT bin-packing, spot/ondemand comparison, egress/storage pricing, and ranked savings reports.
- OpenTelemetry OTLP/HTTP+JSON exporter (`OtlpExporter`) honoring `OTEL_EXPORTER_OTLP_ENDPOINT`; `fish build` exports root + per-task spans at completion.
- Distributed Trace Aggregation (`merge_worker_traces`) â€” dedup, trace-id adoption, orphan re-parenting across workers.
- Build Regression Alerts â€” median-baseline evaluation over rolling JSONL history, surfaced by `fish build`.
- Spot Instance Optimization (`PreemptionRetryExecutor`) â€” retries infrastructure-shaped failures then migrates to an on-demand fallback.
- Plugin Capability Auditor â€” risk-ranked static analysis of wasm plugin manifests.
- Live OSV advisory feed (`OsvClient`, `FISH_OSV_ENDPOINT`) â€” batched querybatch lookups replacing the stale embedded snapshot for Cargo and npm.
- RBAC resource-scoped target rules (e.g. `prod/*` requiring higher clearance) and append-only JSONL audit log.
- CAS synchronous reader (`with_artifact_bytes`).
- Web Dashboard JSONL persistence (`PersistentMetricsStore`) and `/api/team-stats`.
- K8s FishCluster CRD manifests, reconciler, and spot-node handling in the Go control plane.
- Cross-region replication topology (`ReplicationTopology`) with region-aware catalog tracking and TTL eviction.
- Signature Gate (`SignedArtifactGate`) â€” Ed25519 verify-on-read for remote artifacts, wired into `fish build` via `FISH_SIGNING_SEED`.

#### v0.6
- Toolchain provisioning: hermetic downloader with network fetch, SHA-256 checksums, tar/zip extraction; `fish.lock` lockfile with drift verification.
- Sandbox presets (`strict`/`default`/`trusted`), drift detector over BLAKE3 fingerprints.
- Build reproducibility: trace replay certification (`ExecutionTrace` save/load/replay), bit-for-bit output comparison (`certify_reproducible`).
- SLSA in-toto CLI (`fish attest`) generating Statement/v1 per output artifact.
- Multi-tenant CAS with tenant namespacing and per-team byte quotas.
- Plugin Marketplace: registry fetch/search/sign/install with Ed25519 verification.

#### Long-term (v1.0+)
- MicroVM hardware isolation config generator (`MicroVmConfig`, Firecracker JSON emission, VM lifecycle state machine) in `fish-sandbox`.
- HA Coordinator Raft consensus in Go (`go/pkg/raft`): leader election, log replication, conflict truncation, committed-entry application.
- Cross-language AST sub-tree caching: Rust function boundary detection, BLAKE3 per-function hashing, changed-vs-unchanged diffing (`fish-incremental/src/subtree_cache.rs`).
- Global P2P mesh gossip discovery with dedup loop prevention on top of the replication topology.
- Federated build grids (`BuildGrid`) with LocalityFirst/RoundRobin/LeastLoaded routing policies.

### Changed
- Wasm plugin engine embeds a real wasmi runtime behind the `wasm` feature flag; hooks compile, instantiate, and call exported functions instead of fabricating results.

### Fixed
- Remote CAS downloads now verify BLAKE3 integrity; `stats()`/`list()` propagate errors instead of fabricating zeros.
- CAS GC rewritten for the real sharded layout; chunk reconstruction verifies per-chunk hashes and lengths.
- Windows LLD linking uses `-C linker=lld-link` (the previous flag broke link.exe).
- Response files follow MSVC backslash/quote escaping rules and reject embedded newlines.
- Work-stealing scheduler records real start offsets and worker ids in Chrome traces.
- Racing executor checks cancellation before starting each side and documents duplicated-execution semantics.
- Go Raft election timeout now randomizes correctly over 150â€“300ms (previous string-modulo produced 48â€“57ms); tests use a deterministic timeout override.
- fs watcher sets its running flag only after a successful start and honors the configured debounce interval.
- Jobserver pool clamps limit to â‰¥ 1; resource governor warning threshold no longer saturates at low limits.

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
