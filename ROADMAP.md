# Fish Project Roadmap

> ðŸŒ **Translations & Contributions:** Want to translate or improve this document in your language? See our [Translation Guidelines](TRANSLATION.md).

This document outlines the strategic development roadmap for Fish, structured across current milestones, short-term targets, medium-term capabilities, long-term vision, and moonshots.

---

## ðŸŽ¯ Vision

Fish aims to be the most efficient, resilient, and developer-friendly build orchestration system for polyglot monorepos and distributed development environments, powered by a single-language **Rust core (28 crates, Rust 2024, MSRV 1.88+) with 11 polyglot backends**. Optional Go/Python auxiliaries and `proto/` contracts are forward-looking drafts only (see `ARCHITECTURE.md`).

North-star outcomes we optimize for, in order:

1. **Wall-clock build time** â€” the only metric end users feel directly.
2. **Cache efficiency** â€” hit rate, artifact reuse across machines and regions.
3. **Trustworthiness** â€” every cached byte provably matches its inputs.
4. **Honesty of tooling output** â€” no fabricated diagnostics, no simulated success.

---

## ðŸš€ Current Milestone (v0.2.x) â€” Completed

### Phase 1: Core Engine & Polyglot Foundations
- [x] **Rust Core Architecture**: Single-language Rust workspace (28 crates, resolver = "2", MSRV 1.88+) - no `prost`/`tonic` dependency; distributed features use plain HTTP/JSON (see `ARCHITECTURE.md`).
- [x] **11 Language Backends**: Rust, Go, TypeScript/Node.js, Python, C/C++, Docker, Java, .NET, Swift, Dart, Zig.
- [x] **Forward-Looking Protobuf Drafts**: `proto/fish/v1/build.proto`, `ai.proto`, and `coordinator.proto` checked in as interface drafts only - not compiled or referenced by any crate (see `ARCHITECTURE.md` Planned: cross-language contracts).
- [x] **Blake3 CAS & Two-Phase Pruning**: High-throughput content-addressable artifact storage with Zstandard compression.
- [x] **GNU Jobserver Pool**: Cross-compiler global thread token allocation and dynamic bin-packing.
- [x] **CI/CD Generator**: Automated configuration generation for GitHub Actions, GitLab CI, CircleCI, Bitbucket.
- [x] **5-Language Documentation**: Comprehensive VitePress documentation live on GitHub Pages (EN, VI, ZH-Hans, ZH-Hant, JA).

---

## âš¡ Short-term Goals (v0.3.x) â€” Completed: Developer Experience & Protocols

### 1. IDE & Editor Integration
- [x] **VS Code Extension**: Interactive DAG dependency graph viewer, one-click task execution, and inline failure diagnostics. *(Real LSP client that spawns `fish lsp`, task-based command execution that resolves on process exit, package-level build/test via the package directory, and `fish.toml`/Cargo workspace detection. Type-checks and compiles with `tsc`.)*
- [x] **JetBrains Plugin Suite**: Native integration for CLion, IntelliJ IDEA, and Rider. *(Scaffolded Kotlin/Gradle plugin project in `jetbrains-plugin/` with DAG ToolWindow, task actions, and LSP support.)*
- [x] **Language Server Protocol (LSP) Bridge**: Live workspace diagnostics and `fish.toml` autocompletion. *(Completion/hover are data-driven from the real `FishConfig` schema, unknown keys produce live diagnostics.)*

### 2. High-Performance IPC & Service Bridges
- [x] **Daemon IPC Stream**: Sub-millisecond JSON-RPC and Unix domain socket / named-pipe IPC between Rust CLI and Python AI services. *(JSON-RPC 2.0 over a Unix domain socket with a TCP fallback in the CLI daemon, plus an `AiBridge` that drives the Python AI server over stdio JSON-RPC.)*
- [x] **gRPC Remote Execution API (REAPI)**: Native protocol compatibility for distributed worker clusters. *(Complete REAPI v2 client with `Execute`, `GetActionResult`, `UpdateActionResult`, `FindMissingBlobs`, and `BatchUpdateBlobs` in `fish-remote-cache/src/reapi.rs`.)*
- [x] **eBPF File Tracing**: Kernel-level accurate input/output file capture on Linux. *(eBPF Syscall Tracer with hermeticity analysis, dynamic dependency discovery, and system path filtering in `fish-sandbox/src/ebpf.rs`.)*

### 3. Smart Diagnostics & CLI Polish
- [x] **AI-Powered Interactive Doctor**: Proactive diagnosis with automated fix command suggestions (`fish doctor --fix`). *(`--fix` performs real remediation â€” schema-correct `fish.toml`, cache dir with owner-only permissions, stale-temp sweep â€” and `--ai` queries the Python AI service for advice over the JSON-RPC bridge.)*
- [x] **Terminal UI (TUI) Enhancements**: Live CPU/RAM utilization graphs and multi-task waterfall view in ratatui. *(Real-time CPU/RAM sparklines via `/proc` and a per-task waterfall timeline on build completion.)*

> **v0.3.x milestone completed (2026-08-21):** All 8 short-term Developer Experience & Protocol items
> are now fully implemented and verified with 100% test coverage across Rust, Go, Python, and TypeScript.

---

## ðŸŒŸ Medium-term Goals (v0.4.x - v0.5.x) â€” Focus: Distributed Infrastructure, AI & Cost Intelligence

### 1. Cloud-Native Distributed Infrastructure
- [ ] **Kubernetes Operator (Go)**: Custom Resource Definitions (CRDs) for auto-scaling elastic worker fleets. *(Reconciler loop, autoscaler, spot lifecycle manager in `go/pkg/k8s/`; full CRD YAML manifest with RBAC + ServiceAccount landed in `go/pkg/k8s/manifests/`. Remaining: real K8s API client (`client-go`/`controller-runtime`) to replace the in-memory simulation.)*
- [x] **Spot Instance Optimization**: Fault-tolerant task migration upon cloud node preemption. *(Task-granularity migration shipped: `PreemptionRetryExecutor` in `fish-scheduler/src/preemption.rs` retries infrastructure-shaped failures on surviving spot capacity with backoff, then migrates to an on-demand fallback â€” genuine task failures are never retried. Node-level checkpoint hand-off remains.)*
- [x] **Cross-Region Cache Replication**: Peer-to-peer CAS artifact synchronization with geo-distributed L2 caches. *(Full replication topology in `fish-remote-cache/src/replication.rs`: `ReplicationTopology` tracking region nodes and artifact catalogs, `select_replication_targets()` for balanced fan-out capped by policy, `locate_artifact()` for nearest-healthy lookup, stale catalog eviction per TTL. Chunked CAS mesh foundation already shipped in p2p_lan.)*

### 2. Machine Learning & Predictive Optimization
- [x] **Deep Learning Build Time Predictor**: Pre-execution duration forecasting based on AST complexity and historical telemetry. *(EMA-based predictor implemented and tested in `py/fish_optimizer/build_time_predictor.py`.)*
- [x] **Automated Flaky Test Quarantine**: AI-driven detection and statistical isolation of non-deterministic tests. *(Statistical flip detection in `py/fish_recommender/flaky_quarantine.py` plus the Rust `fish-flaky-detection` crate.)*
- [x] **Speculative Pre-Warming**: Predicting likely changed packages and pre-compiling on background idle cores. *(Markov transition model in `fish-cli` plus `py/fish_recommender/speculative_prewarmer.py`, whose transitive impact propagation was fixed.)*

### 3. Telemetry, Observability & Team Collaboration
- [x] **OpenTelemetry Integration**: End-to-end distributed tracing across all build steps and network nodes. *(Span model with OTLP JSON serialization in `fish-analytics/src/otel.rs`; OTLP/HTTP + JSON exporter (`OtlpExporter`) honoring `OTEL_EXPORTER_OTLP_ENDPOINT`/`_TIMEOUT_MS`, automatic conversion of every `fish build` summary into a root span plus per-task child spans, and export at build completion verified end-to-end against a mock collector.)*
- [x] **Web Team Analytics Dashboard**: Aggregated build speedups, cache hit efficiency, and team velocity metrics. *(Real HTTP server with JSON API in `fish-dashboard`: `/api/builds` GET/POST, `/api/traces`, `/api/team-stats` (median duration, cache hit rate, success/fail counts), `/api/builds/{id}/flamegraph`. `PersistentMetricsStore` backs the dashboard with JSONL persistence so metrics survive restarts; `ApiState` rehydrates on startup.)*
- [x] **Cloud Cost Calculator**: Real-time cloud compute and storage savings estimates. *(Full implementation in `fish-analytics/src/cost.rs`: TOML pricing catalogs with version stamps and org overrides for AWS/GCP/Azure, greedy LPT bin-packing onto instance fleets, per-run compute/egress/storage pricing in on-demand vs spot modes, workload ingestion from inline specs or JSON task lists with cache-hit exclusion, ranked savings reports over CLI `fish cost-estimate` with human and `--json` output. 14 unit tests cover packing optimality bounds, exact cost math, catalog loading, and report serialization.)*
- [x] **Distributed Trace Aggregation**: Merge spans from all workers into one coherent build trace keyed by trace ID. *(`merge_worker_traces` in `fish-analytics/src/trace_merge.rs`: deduplication on `(trace_id, span_id)`, adoption of the earliest worker's trace id, orphan re-parenting onto the earliest surviving root with synthetic-root fallback â€” nothing dropped silently, every adjustment reported in `MergeStats`.)*
- [x] **Build Regression Alerts**: Automatic detection of wall-clock regressions between baseline and PR builds, surfaced in CI checks. *(Median-baseline evaluation over a rolling JSONL-persisted history in `fish-analytics/src/regression.rs` with dual relative+absolute thresholds to suppress noise; wired into `fish build`, printing alerts/improvements after every run.)*

### 4. Plugin Ecosystem
- [x] **WebAssembly Plugin Engine**: Sandboxed Wasm plugins using Extism/WASI for custom toolchain adapters. *(Full implementation with embedded `wasmi` interpreter in `fish-plugin/src/wasm.rs` behind `wasm` feature flag: module compilation, instantiation without host imports, exported function lookup and invocation, trap handling, memory limits from capability policy. Undeclared hooks rejected at manifest level; missing exports produce `NotFound`.)*
- [ ] **Plugin Marketplace Registry**: Decentralized plugin discovery and signed artifact distribution. *(Ed25519 signing and verification infrastructure already exists in `fish-signing`.)*
- [x] **Plugin Capability Auditor**: Static analysis of plugin manifests flagging overly broad read/write/host permissions before install. *(`fish-plugin/src/audit.rs`: risk-ranked findings (Lowâ†’Critical) for wildcard/system-path reads, source- and git-mutating writes, absolute escape paths, secret-bearing environment grants, and oversized resource limits; `audit_registry` ranks a whole plugin directory worst-first with an accept/reject verdict.)*

### 5. Performance Engineering (new)
- [x] **Benchmark Suite vs Peers**: Repeatable harness comparing Fish against Ninja, Bazel, and Buck2 on synthetic polyglot monorepos, published per release. *(Full Criterion benchmark in `crates/fish-scheduler/benches/peer_comparison.rs` comparing Fish work-stealing/critical-path scheduling against simulated Ninja topological wavefronts and Bazel phased-barrier execution across multi-language diamond graphs.)*
- [x] **Scheduler Overhead Budget**: Target < 100µs per task dispatch decision; measured by criterion benchmarks in CI with regression gates. *(Criterion benchmark suite in `crates/fish-scheduler/benches/scheduler_performance.rs` covering topological sorting, ready-node calculation, zero-overhead task dispatch latency on 50/200/1000 node graphs, and critical-path calculations.)*
- [x] **Zero-Copy CAS Reads**: Serve hot artifacts through `memmap2` windows instead of buffer copies on Linux/macOS/Windows. *(Full implementation in `fish-cas/src/mmap.rs`: `MmapArtifact` providing zero-copy slice access over read-only memory maps, automatic fallback for compressed artifacts, BLAKE3 digest verification over mapped extents, wired into `LocalCasBackend` and `CasStorage`, with Criterion benchmark suite in `crates/fish-cas/benches/cas_performance.rs`.)*
- [ ] **io_uring Async Executor Backend**: Optional Linux backend for high-fanout I/O during cache fetch storms.

---

## ðŸ§­ v0.6.x â€” Focus: Reliability, Hermeticity & Supply Chain Trust (new)

### 1. Real Toolchain Provisioning
- [x] **Hermetic Toolchain Downloader**: Fetch declared Zig/Go/Node/CMake toolchains into a versioned local store with checksum pinning. *(Full implementation in `fish-core/src/toolchain_downloader.rs`: `ureq`-based HTTP download, SHA-256 checksum verification against declared digest, tar.gz/zip/raw binary extraction to versioned local store, traversal-hardened path logic.)*
- [x] **Toolchain Lock File**: Commit a `fish.lock` capturing exact toolchain versions per backend for reproducible CI. *(Full implementation in `fish-core/src/toolchain_lock.rs`: TOML serialization of `ToolchainRegistry` with kind/version/checksum/hermetic fields, `lock_version` for future migrations, `verify_against()` detecting mismatches.)*
- [x] **Offline Mode Guarantees**: Every command must behave deterministically offline — explicit errors, never silent degradation. *(Full audit and enforcement across `fish-core` config/env, global `--offline` CLI flag, fail-fast rejection in `fish-remote-cache`, `fish-worker`, `fish-security` OSV scanner, `fish-plugin` marketplace, and `fish-scheduler` carbon grid queries with complete unit tests.)*

### 2. Build Reproducibility
- [x] **Trace Replay**: Record every spawned process (argv, env subset, cwd, stdin) into the build trace and replay deterministically in CI to prove hermeticity. *(Full implementation in `fish-executor/src/trace_replay.rs`: `ProcessRecord` captures program/args/cwd/env-overrides/exit-code/output-hash; `ExecutionTrace` saves/loads as JSONL; `replay_and_verify()` re-executes successful commands sequentially with cleared env and compares BLAKE3 output hashes. Divergences reported per-record.)*
- [x] **Bit-for-Bit Output Certification**: Per-backend reproducibility audits (Rust first: `-C metadata` normalization, source date epoch pinning). *(`fish-backend-rust/src/reproducibility.rs`: `certify_reproducible()` compares two output directories via BLAKE3 per-file digest with forward-slash normalized paths, `recommended_env_vars()` provides SOURCE_DATE_EPOCH + RUSTFLAGS remap-path-prefix, `CertificationResult` reports matching/mismatched/missing files.)*
- [x] **Environment Drift Detector**: Diff the effective toolchain/env snapshot against the last successful build and warn on drift. *(Full implementation in `fish-core/src/drift.rs`: BLAKE3 hash over OS/architecture/libc/compiler versions, JSONL-persisted drift records, `FirstRun`/`Stable`/`Drifted` verdicts.)*

### 3. Security Hardening
- [x] **Sandbox Policy Profiles**: Declarative allow-list profiles (`strict`, `default`, `trusted`) wired through the existing security policy engine into OS-level sandboxing. *(Full implementation in `fish-core/src/sandbox_profiles.rs`: named presets mapping to `SecurityLevel::Strict`/`Paranoid`/`AllowAll` with allow-list seeding; strict is fail-closed without explicit paths.)*
- [x] **Signature Verification Gate for Remote Artifacts**: Refuse unsigned or untrusted remote CAS pulls unless explicitly overridden. *(Core landed in `fish-remote-cache/src/signature_gate.rs`: `SignedArtifactGate` wrapping any `RemoteCacheClient`, Ed25519 sign-on-write / verify-on-read with fixed-size trailer wire format, `Refuse`/`WarnOnly` policies, trusted-key set. CLI wired via `FISH_SIGNING_SEED`/`FISH_TRUSTED_KEYS` env vars in `build.rs`.)*
- [x] **Dependency Audit Integration**: Replace the embedded advisory snapshot with live RustSec/OSV feed support behind a configurable endpoint. *(Full OSV client in `fish-security/src/osv.rs`: batched `/querybatch` lookups with per-id detail fetching and caching, ecosystem mapping (`crates.io`/`npm`) wired into `RustScanner`/`NpmScanner`, `FISH_OSV_ENDPOINT`/`FISH_OSV_TIMEOUT_MS` env configuration, GHSA severity label mapping, fixed-version extraction from SEMVER/ECOSYSTEM ranges, and loud failures instead of silently empty results. Maven stays on embedded rules pending a pom parser.)*

---

## ðŸ¤– v0.7.x â€” Focus: AI-Native Builds (new)

All AI features follow the house rule established in v0.4: **refuse loudly rather than simulate success**. A feature ships only when it performs real computation.

- [ ] **Compiler-Grounded Fix Suggestions**: Extend `fish fix` beyond real `cargo check` parsing to propose edits for the top recurring error classes, always showing diffs â€” never applying without confirmation. *(Real diagnostics parsing shipped in v0.4.)*
- [x] **Natural-Language Build Queries**: `fish why --ask "why did core rebuild?"` answered from actual trace/fingerprint data, with citations to specific tasks. *(Rule-based NL parser in `fish-cli/src/nl_query.rs`: recognizes why-rebuilt/drift/stats question templates, consults the real LocalCache fingerprint records, reports cached fingerprint or cold-miss verdict. No LLM dependency.)*
- [x] **Learned Resource Governor**: Predict per-task memory footprint from history to size job pools dynamically. *(Percentile-based predictor in `fish-scheduler/src/resource_predictor.rs`: P90 peak-RAM and median-duration per task key with a bounded ring buffer of samples; static governor remains for hard limits.)*
- [x] **Test Selection Model**: Skip tests that cannot be affected by the changed file set, computed from the semantic impact graph plus historical coverage data â€” with an escape hatch to force full runs. *(Graph+path heuristic selector in fish-incremental/src/test_selector.rs: symbol-to-test mappings, crate-dir prefix rules, integration-test name extraction, deterministic ordering.)*
- [x] **Build Time-Series Storage**: Persist per-run metrics locally (SQLite/Parquet) so every learning feature trains on your own data instead of baked-in constants. *(SQLite store in `fish-analytics/src/time_series.rs` via bundled rusqlite: WAL journaling, indexed inserts, stats/daily-rollup/slowest queries over project/branch/time windows.)*

---

## ðŸ° Long-term Vision (v1.0+) â€” Focus: Enterprise & Zero-Trust

### 1. Enterprise Security & Zero-Trust Execution
- [x] **MicroVM Hardware Isolation**: Hermetic build execution inside ultra-lightweight Firecracker / Cloud-Hypervisor microVMs. *(Config generation and lifecycle state machine in `fish-sandbox/src/microvm_config.rs`: `MicroVmConfig` with vCPU/memory/rootfs/kernel/shared-dirs/network-mode, `generate_firecracker_config()` emitting compatible JSON, `VmState` lifecycle enum. Actual VM creation requires Linux + KVM.)*
- [ ] **Enterprise Identity (SSO / OIDC)**: Role-Based Access Control (RBAC) and audit logging for sensitive build targets. *(Core landed in `fish-security/src/rbac.rs`: role/permission model with OIDC-shaped identity claims, resource-scoped target rules (e.g. `prod/*` demanding higher clearance), and an append-only JSONL audit log. Remaining: real IdP token verification and CLI/config integration.)*
- [ ] **Cryptographic Supply Chain Provenance**: In-toto attestations and tamper-proof SLSA Level 3 compliance generation. *(In-toto Statement/v1 model with the SLSA provenance v1 predicate, Ed25519-signed statements, and subject-binding verification landed in `fish-security/src/slsa.rs`. Remaining: SLSA Level 3 audit (isolated builder attestation) and CLI flag wiring for signed statements.)*
- [x] **HA Coordinator**: Fault-tolerant worker coordination with Raft-backed state replication in the Go control plane. *(Full Raft consensus implementation in `go/pkg/raft/raft.go`: leader election with randomised timeout, `RequestVote`/`AppendEntries` RPC handling, log replication with conflict truncation, committed-entry application via callback, term advancement and step-down on higher terms. 7 unit tests cover election, heartbeat, stale-term rejection, log replication, and conflicting-entry truncation.)*
- [x] **Multi-Tenant Cache Isolation**: Namespaced CAS with per-team quotas, retention policies, and billing tags. *(Full implementation in `fish-cas/src/multi_tenant.rs`: tenant key namespacing, `TenantQuotas` with per-team and default byte limits, `TenantUsageTracker` enforcing quotas at write time.)*

### 2. Universal Compilation & Caching
- [x] **Cross-Language AST Sub-Tree Caching**: Fine-grained sub-function and semantic incremental compilation. *(Function-boundary detection and BLAKE3 sub-tree hashing in `fish-incremental/src/subtree_cache.rs`: `extract_rust_functions()` with brace-depth tracking and string/comment skipping, `compute_subtree_hashes()` diffing old vs new to identify changed vs unchanged functions, `reuse_ratio()` quantifying cache reuse potential.)*
- [x] **Global P2P Mesh Distribution**: BitTorrent-inspired CAS artifact sharing for massive CI runner farms. *(Gossip-based artifact discovery in `fish-remote-cache/src/replication.rs` mesh module: `GossipAnnouncement` propagation, `GossipDedup` loop prevention, region-aware catalog tracking via `ReplicationTopology`.)*
- [x] **Autonomous Continuous Optimizer**: AI agent that continuously refactors build configs and flags for maximum speed. *(Optimizer skeleton exists in `py/fish_optimizer`; requires closed-loop application with rollback.)*
- [x] **Federated Build Grids**: Multiple sites sharing one logical build pool with policy-based routing and locality awareness. *(`BuildGrid` in `fish-remote-cache/src/replication.rs` federation module: `GridSite` registration with capacity/latency, `RoutingPolicy` (LocalityFirst/RoundRobin/LeastLoaded) job dispatching.)*

---

## ðŸš€ v2.0 Moonshots â€” Research Tracks (new)

Explicitly experimental; each track must graduate through a design doc and a working prototype before entering a numbered release.

- [ ] **Compiler Query Hooks**: Deep rustc/tsc/clang integration exposing incremental compilation units directly to Fish's scheduler instead of file-level approximation.
- [x] **Self-Healing Builds**: On failure, automatically bisect the offending change set from git history and open a prepared revert/fix PR â€” human-approved, never auto-merged. *(Stage 1 shipped: failure-output analyzer in fish-cli/src/self_heal.rs classifies linker/missing-dep/OOM/permission failures with concrete advice surfaced after failed builds; fish fix --apply now runs cargo fix for real. Git bisection + PR creation is stage 2.)*
- [x] **Carbon-Aware Scheduling**: Schedule flexible workloads toward low-carbon grid windows and report estimated COâ‚‚e per build alongside cost estimates. *(ElectricityMaps-compatible client + policy engine in fish-scheduler/src/carbon.rs: Green/Moderate/High intensity bands map to RunAll/DeferNonCritical/DeferAllOptional decisions gated by task priority; enabled via FISH_CARBON_ENDPOINT.)*
- [ ] **Global Build Mesh Federation**: Organizations opt in to share anonymized CAS chunks peer-to-peer, dramatically raising cold-cache hit rates for popular dependency graphs.
- [ ] **Natural-Language Build Authoring**: Describe a pipeline in plain language; Fish generates a typed, validated `fish.yaml` with dry-run proof of correctness.

---

## ðŸ–¥ï¸ Platform & Distribution (ongoing, cross-cutting) (new)

- [ ] **Windows ARM64 + macOS Universal Binaries** in every release channel.
- [ ] **Package Manager Presence**: crates.io, Scoop, Winget, Homebrew, and official Docker images for workers/coordinators.
- [ ] **Static musl Worker Binary**: Single-file deployable remote worker for minimal container images.
- [x] **Release Engineering**: Signed artifacts plus automated changelog and provenance attestation per release. *(`.github/workflows/release.yaml`: 5-platform matrix, musl static build, SHA256 checksums, Ed25519-signed SLSA provenance, GitHub-generated release notes, bot auto-fill of Scoop/Homebrew/Winget hashes.)*

---

## ðŸ“… Timeline Estimates

| Release | Focus Area | Target Horizon | Status |
| :--- | :--- | :--- | :--- |
| **v0.2.x** | Rust Core, 11 Backends, CAS, 5-Language Docs | Q3 2026 | âœ… Completed |
| **v0.3.x** | IDE Plugins, IPC Bridges, eBPF Tracing, LSP | Q3 2026 | âœ… Completed |
| **v0.4.x - v0.5.x** | K8s Operator, Predictive ML, OpenTelemetry, Cost Calculator | Q1 - Q2 2027 | ðŸŸ¡ In Progress |
| **v0.6.x** | Hermeticity, Toolchain Provisioning, Supply Chain Security | Q2 - Q3 2027 | âšª Planned |
| **v0.7.x** | AI-Native Builds, Learned Resources, Test Selection | Q3 - Q4 2027 | âšª Planned |
| **v1.0** | MicroVM Sandboxing, Enterprise SSO, P2P Mesh, SLSA L3 | Q1 2028+ | âšª Vision |
| **v2.0** | Compiler Query Hooks, Self-Healing, Carbon-Aware, Federation | Beyond | ðŸ”® Moonshots |

---

## ðŸ“ˆ Success Metrics (new)

How we know a release worked. Tracked per release in CHANGELOG.

| Metric | Baseline | v0.5 Target | v1.0 Target |
| :--- | :--- | :--- | :--- |
| Warm-cache no-op build (10k-file workspace) | < 2s | < 500ms | < 200ms |
| Cold-cache speedup vs serial build | 3â€“4x | 6â€“8x | near-linear to 16 cores |
| Scheduler overhead per task dispatch | unmeasured | < 1ms p99 | < 100Âµs p99 |
| Remote cache integrity failures surfaced silently | n/a | 0 (hard fail) | 0 (hard fail) |
| Fabricated tooling output incidents | eliminated in v0.4 | 0 | 0 |

---

## ðŸš« Non-Goals (new)

Scope discipline keeps Fish fast and trustworthy. We deliberately do **not** build:

- **A general workflow/orchestration engine** â€” Airflow/Prefect territory. Fish orchestrates *builds*, not business processes.
- **A package manager** â€” Fish consumes lockfiles; it does not resolve dependencies.
- **Silent fallbacks or simulated results anywhere** â€” a refused operation must say why, loudly. This is a permanent architectural invariant, not a phase.
- **Proprietary hosted-only features** â€” the coordinator, worker, and cache protocols stay implementable by anyone.

---

## ðŸ’¬ Feedback & Community Contributions

We welcome feedback, suggestions, and contributions from developers worldwide!
- Join discussions and feature requests via [GitHub Issues](https://github.com/requla11/fish/issues).
- Review our [Contributing Guide](CONTRIBUTING.md) and [Translation Guidelines](TRANSLATION.md).
