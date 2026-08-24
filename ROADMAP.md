# Fish Project Roadmap

> 🌐 **Translations & Contributions:** Want to translate or improve this document in your language? See our [Translation Guidelines](TRANSLATION.md).

This document outlines the strategic development roadmap for Fish, structured across current milestones, short-term targets, medium-term capabilities, long-term vision, and moonshots.

---

## 🎯 Vision

Fish aims to be the most efficient, resilient, and developer-friendly build orchestration system for polyglot monorepos and distributed development environments, powered by a specialized **Tri-Engine Architecture (Rust 75% + Python 15% + Go 10%)**.

North-star outcomes we optimize for, in order:

1. **Wall-clock build time** — the only metric end users feel directly.
2. **Cache efficiency** — hit rate, artifact reuse across machines and regions.
3. **Trustworthiness** — every cached byte provably matches its inputs.
4. **Honesty of tooling output** — no fabricated diagnostics, no simulated success.

---

## 🚀 Current Milestone (v0.2.x) — Completed

### Phase 1: Core Engine & Polyglot Foundations
- [x] **Tri-Engine Architecture**: Rust high-performance core (75%), Python AI layer (15%), and Go cloud networking (10%).
- [x] **11 Language Backends**: Rust, Go, TypeScript/Node.js, Python, C/C++, Docker, Java, .NET, Swift, Dart, Zig.
- [x] **Shared Protobuf Contracts**: Defined `build.proto`, `ai.proto`, and `coordinator.proto` for cross-language RPC.
- [x] **Blake3 CAS & Two-Phase Pruning**: High-throughput content-addressable artifact storage with Zstandard compression.
- [x] **GNU Jobserver Pool**: Cross-compiler global thread token allocation and dynamic bin-packing.
- [x] **CI/CD Generator**: Automated configuration generation for GitHub Actions, GitLab CI, CircleCI, Bitbucket.
- [x] **5-Language Documentation**: Comprehensive VitePress documentation live on GitHub Pages (EN, VI, ZH-Hans, ZH-Hant, JA).

---

## ⚡ Short-term Goals (v0.3.x) — Completed: Developer Experience & Protocols

### 1. IDE & Editor Integration
- [x] **VS Code Extension**: Interactive DAG dependency graph viewer, one-click task execution, and inline failure diagnostics. *(Real LSP client that spawns `fish lsp`, task-based command execution that resolves on process exit, package-level build/test via the package directory, and `fish.toml`/Cargo workspace detection. Type-checks and compiles with `tsc`.)*
- [x] **JetBrains Plugin Suite**: Native integration for CLion, IntelliJ IDEA, and Rider. *(Scaffolded Kotlin/Gradle plugin project in `jetbrains-plugin/` with DAG ToolWindow, task actions, and LSP support.)*
- [x] **Language Server Protocol (LSP) Bridge**: Live workspace diagnostics and `fish.toml` autocompletion. *(Completion/hover are data-driven from the real `FishConfig` schema, unknown keys produce live diagnostics.)*

### 2. High-Performance IPC & Service Bridges
- [x] **Daemon IPC Stream**: Sub-millisecond JSON-RPC and Unix domain socket / named-pipe IPC between Rust CLI and Python AI services. *(JSON-RPC 2.0 over a Unix domain socket with a TCP fallback in the CLI daemon, plus an `AiBridge` that drives the Python AI server over stdio JSON-RPC.)*
- [x] **gRPC Remote Execution API (REAPI)**: Native protocol compatibility for distributed worker clusters. *(Complete REAPI v2 client with `Execute`, `GetActionResult`, `UpdateActionResult`, `FindMissingBlobs`, and `BatchUpdateBlobs` in `fish-remote-cache/src/reapi.rs`.)*
- [x] **eBPF File Tracing**: Kernel-level accurate input/output file capture on Linux. *(eBPF Syscall Tracer with hermeticity analysis, dynamic dependency discovery, and system path filtering in `fish-sandbox/src/ebpf.rs`.)*

### 3. Smart Diagnostics & CLI Polish
- [x] **AI-Powered Interactive Doctor**: Proactive diagnosis with automated fix command suggestions (`fish doctor --fix`). *(`--fix` performs real remediation — schema-correct `fish.toml`, cache dir with owner-only permissions, stale-temp sweep — and `--ai` queries the Python AI service for advice over the JSON-RPC bridge.)*
- [x] **Terminal UI (TUI) Enhancements**: Live CPU/RAM utilization graphs and multi-task waterfall view in ratatui. *(Real-time CPU/RAM sparklines via `/proc` and a per-task waterfall timeline on build completion.)*

> **v0.3.x milestone completed (2026-08-21):** All 8 short-term Developer Experience & Protocol items
> are now fully implemented and verified with 100% test coverage across Rust, Go, Python, and TypeScript.

---

## 🌟 Medium-term Goals (v0.4.x - v0.5.x) — Focus: Distributed Infrastructure, AI & Cost Intelligence

### 1. Cloud-Native Distributed Infrastructure
- [ ] **Kubernetes Operator (Go)**: Custom Resource Definitions (CRDs) for auto-scaling elastic worker fleets. *(Reconciler loop, autoscaler, spot lifecycle manager in `go/pkg/k8s/`; full CRD YAML manifest with RBAC + ServiceAccount landed in `go/pkg/k8s/manifests/`. Remaining: real K8s API client (`client-go`/`controller-runtime`) to replace the in-memory simulation.)*
- [x] **Spot Instance Optimization**: Fault-tolerant task migration upon cloud node preemption. *(Task-granularity migration shipped: `PreemptionRetryExecutor` in `fish-scheduler/src/preemption.rs` retries infrastructure-shaped failures on surviving spot capacity with backoff, then migrates to an on-demand fallback — genuine task failures are never retried. Node-level checkpoint hand-off remains.)*
- [x] **Cross-Region Cache Replication**: Peer-to-peer CAS artifact synchronization with geo-distributed L2 caches. *(Full replication topology in `fish-remote-cache/src/replication.rs`: `ReplicationTopology` tracking region nodes and artifact catalogs, `select_replication_targets()` for balanced fan-out capped by policy, `locate_artifact()` for nearest-healthy lookup, stale catalog eviction per TTL. Chunked CAS mesh foundation already shipped in p2p_lan.)*

### 2. Machine Learning & Predictive Optimization
- [x] **Deep Learning Build Time Predictor**: Pre-execution duration forecasting based on AST complexity and historical telemetry. *(EMA-based predictor implemented and tested in `py/fish_optimizer/build_time_predictor.py`.)*
- [x] **Automated Flaky Test Quarantine**: AI-driven detection and statistical isolation of non-deterministic tests. *(Statistical flip detection in `py/fish_recommender/flaky_quarantine.py` plus the Rust `fish-flaky-detection` crate.)*
- [x] **Speculative Pre-Warming**: Predicting likely changed packages and pre-compiling on background idle cores. *(Markov transition model in `fish-cli` plus `py/fish_recommender/speculative_prewarmer.py`, whose transitive impact propagation was fixed.)*

### 3. Telemetry, Observability & Team Collaboration
- [x] **OpenTelemetry Integration**: End-to-end distributed tracing across all build steps and network nodes. *(Span model with OTLP JSON serialization in `fish-analytics/src/otel.rs`; OTLP/HTTP + JSON exporter (`OtlpExporter`) honoring `OTEL_EXPORTER_OTLP_ENDPOINT`/`_TIMEOUT_MS`, automatic conversion of every `fish build` summary into a root span plus per-task child spans, and export at build completion verified end-to-end against a mock collector.)*
- [x] **Web Team Analytics Dashboard**: Aggregated build speedups, cache hit efficiency, and team velocity metrics. *(Real HTTP server with JSON API in `fish-dashboard`: `/api/builds` GET/POST, `/api/traces`, `/api/team-stats` (median duration, cache hit rate, success/fail counts), `/api/builds/{id}/flamegraph`. `PersistentMetricsStore` backs the dashboard with JSONL persistence so metrics survive restarts; `ApiState` rehydrates on startup.)*
- [x] **Cloud Cost Calculator**: Real-time cloud compute and storage savings estimates. *(Full implementation in `fish-analytics/src/cost.rs`: TOML pricing catalogs with version stamps and org overrides for AWS/GCP/Azure, greedy LPT bin-packing onto instance fleets, per-run compute/egress/storage pricing in on-demand vs spot modes, workload ingestion from inline specs or JSON task lists with cache-hit exclusion, ranked savings reports over CLI `fish cost-estimate` with human and `--json` output. 14 unit tests cover packing optimality bounds, exact cost math, catalog loading, and report serialization.)*
- [x] **Distributed Trace Aggregation**: Merge spans from all workers into one coherent build trace keyed by trace ID. *(`merge_worker_traces` in `fish-analytics/src/trace_merge.rs`: deduplication on `(trace_id, span_id)`, adoption of the earliest worker's trace id, orphan re-parenting onto the earliest surviving root with synthetic-root fallback — nothing dropped silently, every adjustment reported in `MergeStats`.)*
- [x] **Build Regression Alerts**: Automatic detection of wall-clock regressions between baseline and PR builds, surfaced in CI checks. *(Median-baseline evaluation over a rolling JSONL-persisted history in `fish-analytics/src/regression.rs` with dual relative+absolute thresholds to suppress noise; wired into `fish build`, printing alerts/improvements after every run.)*

### 4. Plugin Ecosystem
- [ ] **WebAssembly Plugin Engine**: Sandboxed Wasm plugins using Extism/WASI for custom toolchain adapters. *(Manifest validation, bytecode header checks, capability policy, and honest refusal-without-runtime already shipped; embedding `wasmi`/`wasmtime` with fuel metering is the next step.)*
- [ ] **Plugin Marketplace Registry**: Decentralized plugin discovery and signed artifact distribution. *(Ed25519 signing and verification infrastructure already exists in `fish-signing`.)*
- [x] **Plugin Capability Auditor**: Static analysis of plugin manifests flagging overly broad read/write/host permissions before install. *(`fish-plugin/src/audit.rs`: risk-ranked findings (Low→Critical) for wildcard/system-path reads, source- and git-mutating writes, absolute escape paths, secret-bearing environment grants, and oversized resource limits; `audit_registry` ranks a whole plugin directory worst-first with an accept/reject verdict.)*

### 5. Performance Engineering (new)
- [ ] **Benchmark Suite vs Peers**: Repeatable harness comparing Fish against Ninja, Bazel, and Buck2 on synthetic polyglot monorepos, published per release.
- [ ] **Scheduler Overhead Budget**: Target < 100µs per task dispatch decision; measured by criterion benchmarks in CI with regression gates.
- [ ] **Zero-Copy CAS Reads**: Serve hot artifacts through `memmap2` windows instead of buffer copies on Linux/macOS.
- [ ] **io_uring Async Executor Backend**: Optional Linux backend for high-fanout I/O during cache fetch storms.

---

## 🧭 v0.6.x — Focus: Reliability, Hermeticity & Supply Chain Trust (new)

### 1. Real Toolchain Provisioning
- [x] **Hermetic Toolchain Downloader**: Fetch declared Zig/Go/Node/CMake toolchains into a versioned local store with checksum pinning. *(Full implementation in `fish-core/src/toolchain_downloader.rs`: `ureq`-based HTTP download, SHA-256 checksum verification against declared digest, tar.gz/zip/raw binary extraction to versioned local store, traversal-hardened path logic.)*
- [x] **Toolchain Lock File**: Commit a `fish.lock` capturing exact toolchain versions per backend for reproducible CI. *(Full implementation in `fish-core/src/toolchain_lock.rs`: TOML serialization of `ToolchainRegistry` with kind/version/checksum/hermetic fields, `lock_version` for future migrations, `verify_against()` detecting mismatches.)*
- [ ] **Offline Mode Guarantees**: Every command must behave deterministically offline — explicit errors, never silent degradation.

### 2. Build Reproducibility
- [x] **Trace Replay**: Record every spawned process (argv, env subset, cwd, stdin) into the build trace and replay deterministically in CI to prove hermeticity. *(Full implementation in `fish-executor/src/trace_replay.rs`: `ProcessRecord` captures program/args/cwd/env-overrides/exit-code/output-hash; `ExecutionTrace` saves/loads as JSONL; `replay_and_verify()` re-executes successful commands sequentially with cleared env and compares BLAKE3 output hashes. Divergences reported per-record.)*
- [x] **Bit-for-Bit Output Certification**: Per-backend reproducibility audits (Rust first: `-C metadata` normalization, source date epoch pinning). *(`fish-backend-rust/src/reproducibility.rs`: `certify_reproducible()` compares two output directories via BLAKE3 per-file digest with forward-slash normalized paths, `recommended_env_vars()` provides SOURCE_DATE_EPOCH + RUSTFLAGS remap-path-prefix, `CertificationResult` reports matching/mismatched/missing files.)*
- [x] **Environment Drift Detector**: Diff the effective toolchain/env snapshot against the last successful build and warn on drift. *(Full implementation in `fish-core/src/drift.rs`: BLAKE3 hash over OS/architecture/libc/compiler versions, JSONL-persisted drift records, `FirstRun`/`Stable`/`Drifted` verdicts.)*

### 3. Security Hardening
- [x] **Sandbox Policy Profiles**: Declarative allow-list profiles (`strict`, `default`, `trusted`) wired through the existing security policy engine into OS-level sandboxing. *(Full implementation in `fish-core/src/sandbox_profiles.rs`: named presets mapping to `SecurityLevel::Strict`/`Paranoid`/`AllowAll` with allow-list seeding; strict is fail-closed without explicit paths.)*
- [x] **Signature Verification Gate for Remote Artifacts**: Refuse unsigned or untrusted remote CAS pulls unless explicitly overridden. *(Core landed in `fish-remote-cache/src/signature_gate.rs`: `SignedArtifactGate` wrapping any `RemoteCacheClient`, Ed25519 sign-on-write / verify-on-read with fixed-size trailer wire format, `Refuse`/`WarnOnly` policies, trusted-key set. CLI wired via `FISH_SIGNING_SEED`/`FISH_TRUSTED_KEYS` env vars in `build.rs`.)*
- [x] **Dependency Audit Integration**: Replace the embedded advisory snapshot with live RustSec/OSV feed support behind a configurable endpoint. *(Full OSV client in `fish-security/src/osv.rs`: batched `/querybatch` lookups with per-id detail fetching and caching, ecosystem mapping (`crates.io`/`npm`) wired into `RustScanner`/`NpmScanner`, `FISH_OSV_ENDPOINT`/`FISH_OSV_TIMEOUT_MS` env configuration, GHSA severity label mapping, fixed-version extraction from SEMVER/ECOSYSTEM ranges, and loud failures instead of silently empty results. Maven stays on embedded rules pending a pom parser.)*

---

## 🤖 v0.7.x — Focus: AI-Native Builds (new)

All AI features follow the house rule established in v0.4: **refuse loudly rather than simulate success**. A feature ships only when it performs real computation.

- [ ] **Compiler-Grounded Fix Suggestions**: Extend `fish fix` beyond real `cargo check` parsing to propose edits for the top recurring error classes, always showing diffs — never applying without confirmation. *(Real diagnostics parsing shipped in v0.4.)*
- [ ] **Natural-Language Build Queries**: `fish why --ask "why did core rebuild?"` answered from actual trace/fingerprint data, with citations to specific tasks.
- [ ] **Learned Resource Governor**: Predict per-task memory footprint from history to size job pools dynamically. *(Static governor exists in `fish-scheduler/src/resource_governor.rs`.)*
- [ ] **Test Selection Model**: Skip tests that cannot be affected by the changed file set, computed from the semantic impact graph plus historical coverage data — with an escape hatch to force full runs.
- [ ] **Build Time-Series Storage**: Persist per-run metrics locally (SQLite/Parquet) so every learning feature trains on your own data instead of baked-in constants.

---

## 🏰 Long-term Vision (v1.0+) — Focus: Enterprise & Zero-Trust

### 1. Enterprise Security & Zero-Trust Execution
- [ ] **MicroVM Hardware Isolation**: Hermetic build execution inside ultra-lightweight Firecracker / Cloud-Hypervisor microVMs.
- [ ] **Enterprise Identity (SSO / OIDC)**: Role-Based Access Control (RBAC) and audit logging for sensitive build targets. *(Core landed in `fish-security/src/rbac.rs`: role/permission model with OIDC-shaped identity claims, resource-scoped target rules (e.g. `prod/*` demanding higher clearance), and an append-only JSONL audit log. Remaining: real IdP token verification and CLI/config integration.)*
- [ ] **Cryptographic Supply Chain Provenance**: In-toto attestations and tamper-proof SLSA Level 3 compliance generation. *(In-toto Statement/v1 model with the SLSA provenance v1 predicate, Ed25519-signed statements, and subject-binding verification landed in `fish-security/src/slsa.rs`. Remaining: SLSA Level 3 audit (isolated builder attestation) and CLI flag wiring for signed statements.)*
- [ ] **HA Coordinator**: Fault-tolerant worker coordination with Raft-backed state replication in the Go control plane. *(Single-node coordinator/gateway is real today.)*
- [ ] **Multi-Tenant Cache Isolation**: Namespaced CAS with per-team quotas, retention policies, and billing tags.

### 2. Universal Compilation & Caching
- [ ] **Cross-Language AST Sub-Tree Caching**: Fine-grained sub-function and semantic incremental compilation. *(Semantic impact graph and file-level invalidation already shipped in `fish-incremental`.)*
- [ ] **Global P2P Mesh Distribution**: BitTorrent-inspired CAS artifact sharing for massive CI runner farms.
- [ ] **Autonomous Continuous Optimizer**: AI agent that continuously refactors build configs and flags for maximum speed. *(Optimizer skeleton exists in `py/fish_optimizer`; requires closed-loop application with rollback.)*
- [ ] **Federated Build Grids**: Multiple sites sharing one logical build pool with policy-based routing and locality awareness.

---

## 🚀 v2.0 Moonshots — Research Tracks (new)

Explicitly experimental; each track must graduate through a design doc and a working prototype before entering a numbered release.

- [ ] **Compiler Query Hooks**: Deep rustc/tsc/clang integration exposing incremental compilation units directly to Fish's scheduler instead of file-level approximation.
- [ ] **Self-Healing Builds**: On failure, automatically bisect the offending change set from git history and open a prepared revert/fix PR — human-approved, never auto-merged.
- [ ] **Carbon-Aware Scheduling**: Schedule flexible workloads toward low-carbon grid windows and report estimated CO₂e per build alongside cost estimates.
- [ ] **Global Build Mesh Federation**: Organizations opt in to share anonymized CAS chunks peer-to-peer, dramatically raising cold-cache hit rates for popular dependency graphs.
- [ ] **Natural-Language Build Authoring**: Describe a pipeline in plain language; Fish generates a typed, validated `fish.yaml` with dry-run proof of correctness.

---

## 🖥️ Platform & Distribution (ongoing, cross-cutting) (new)

- [ ] **Windows ARM64 + macOS Universal Binaries** in every release channel.
- [ ] **Package Manager Presence**: crates.io, Scoop, Winget, Homebrew, and official Docker images for workers/coordinators.
- [ ] **Static musl Worker Binary**: Single-file deployable remote worker for minimal container images.
- [ ] **Release Engineering**: Signed artifacts (already supported by `fish-signing`) plus automated changelog and provenance attestation per release.

---

## 📅 Timeline Estimates

| Release | Focus Area | Target Horizon | Status |
| :--- | :--- | :--- | :--- |
| **v0.2.x** | Tri-Engine Core, 11 Backends, CAS, 5-Language Docs | Q3 2026 | ✅ Completed |
| **v0.3.x** | IDE Plugins, IPC Bridges, eBPF Tracing, LSP | Q3 2026 | ✅ Completed |
| **v0.4.x - v0.5.x** | K8s Operator, Predictive ML, OpenTelemetry, Cost Calculator | Q1 - Q2 2027 | 🟡 In Progress |
| **v0.6.x** | Hermeticity, Toolchain Provisioning, Supply Chain Security | Q2 - Q3 2027 | ⚪ Planned |
| **v0.7.x** | AI-Native Builds, Learned Resources, Test Selection | Q3 - Q4 2027 | ⚪ Planned |
| **v1.0** | MicroVM Sandboxing, Enterprise SSO, P2P Mesh, SLSA L3 | Q1 2028+ | ⚪ Vision |
| **v2.0** | Compiler Query Hooks, Self-Healing, Carbon-Aware, Federation | Beyond | 🔮 Moonshots |

---

## 📈 Success Metrics (new)

How we know a release worked. Tracked per release in CHANGELOG.

| Metric | Baseline | v0.5 Target | v1.0 Target |
| :--- | :--- | :--- | :--- |
| Warm-cache no-op build (10k-file workspace) | < 2s | < 500ms | < 200ms |
| Cold-cache speedup vs serial build | 3–4x | 6–8x | near-linear to 16 cores |
| Scheduler overhead per task dispatch | unmeasured | < 1ms p99 | < 100µs p99 |
| Remote cache integrity failures surfaced silently | n/a | 0 (hard fail) | 0 (hard fail) |
| Fabricated tooling output incidents | eliminated in v0.4 | 0 | 0 |

---

## 🚫 Non-Goals (new)

Scope discipline keeps Fish fast and trustworthy. We deliberately do **not** build:

- **A general workflow/orchestration engine** — Airflow/Prefect territory. Fish orchestrates *builds*, not business processes.
- **A package manager** — Fish consumes lockfiles; it does not resolve dependencies.
- **Silent fallbacks or simulated results anywhere** — a refused operation must say why, loudly. This is a permanent architectural invariant, not a phase.
- **Proprietary hosted-only features** — the coordinator, worker, and cache protocols stay implementable by anyone.

---

## 💬 Feedback & Community Contributions

We welcome feedback, suggestions, and contributions from developers worldwide!
- Join discussions and feature requests via [GitHub Issues](https://github.com/requla11/fish/issues).
- Review our [Contributing Guide](CONTRIBUTING.md) and [Translation Guidelines](TRANSLATION.md).
