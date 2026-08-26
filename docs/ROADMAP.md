# Fish Project Roadmap

> 🌐 **Translations & Contributions:** Want to translate or improve this document in your language? See our [Translation Guidelines](TRANSLATION.md).

This document outlines the strategic development roadmap for Fish, structured across current milestones, short-term targets, medium-term capabilities, and long-term vision.

---

## 🎯 Vision

Fish aims to be the most efficient, resilient, and developer-friendly build orchestration system for polyglot monorepos and distributed development environments.

Today Fish ships as three cooperating layers, all present in this repository:

- **Rust core** (`crates/`) — the CLI and build engine; the primary product surface.
- **Go control plane** (`go/`) — coordinator, worker gateway, DB migrator, and Kubernetes operator binaries with their own test suites.
- **Python AI services** (`py/`) — build-time prediction, flaky-test quarantine, speculative pre-warming, analytics, and an advice server, invoked from the CLI through the JSON-RPC `AiBridge` / daemon IPC.

The shared Protobuf contracts under `proto/` remain design drafts that no crate references yet; cross-service IPC currently speaks JSON-RPC instead.

---

## 🚀 Current Milestone (v0.2.x) — Completed

### Phase 1: Core Engine & Polyglot Foundations
- [x] **Tri-Engine Architecture**: Rust core (`crates/`), Go control plane (`go/`: coordinator, worker gateway, DB migrator, K8s operator), and Python AI services (`py/`). The layers are wired together over JSON-RPC IPC rather than the originally envisioned shared Protobuf contracts.
- [x] **11 Language Backends**: Rust, Go, TypeScript/Node.js, Python, C/C++, Docker, Java, .NET, Swift, Dart, Zig.
- [ ] **Shared Protobuf Contracts**: *Drafts only.* `build.proto`, `ai.proto`, and `coordinator.proto` exist under `proto/` but are referenced by no crate (no gRPC dependencies in the workspace).
- [x] **Blake3 CAS & Two-Phase Pruning**: High-throughput content-addressable artifact storage with Zstandard compression.
- [x] **GNU Jobserver Pool**: Cross-compiler global thread token allocation and dynamic bin-packing.
- [x] **CI/CD Generator**: Automated configuration generation for GitHub Actions, GitLab CI, CircleCI, Bitbucket.
- [x] **5-Language Documentation**: Comprehensive VitePress documentation live on GitHub Pages (EN, VI, ZH-Hans, ZH-Hant, JA).

---

## ⚡ Short-term Goals (v0.3.x) — Focus: Developer Experience & Protocols

### 1. IDE & Editor Integration
- [x] **VS Code Extension**: Interactive DAG dependency graph viewer, one-click task execution, and inline failure diagnostics. *(Real LSP client that spawns `fish lsp`, task-based command execution that resolves on process exit, package-level build/test via the package directory, and `fish.toml`/Cargo workspace detection. Type-checks and compiles with `tsc`.)*
- [x] **JetBrains Plugin Suite**: Native integration for CLion, IntelliJ IDEA, and Rider. *(Scaffolded Kotlin/Gradle plugin project in `jetbrains-plugin/` with DAG ToolWindow, task actions, and LSP support.)*
- [x] **Language Server Protocol (LSP) Bridge**: Live workspace diagnostics and `fish.toml` autocompletion. *(Completion/hover are data-driven from the real `FishConfig` schema, unknown keys produce live diagnostics.)*

### 2. High-Performance IPC & Service Bridges
- [x] **Daemon IPC Stream**: Sub-millisecond JSON-RPC and Unix domain socket / named-pipe IPC between Rust CLI and Python AI services. *(JSON-RPC 2.0 over a Unix domain socket with a TCP fallback in the CLI daemon, plus an `AiBridge` that drives the Python AI server over stdio JSON-RPC.)*
- [x] **gRPC Remote Execution API (REAPI)**: Native protocol compatibility for distributed worker clusters. *(Complete REAPI v2 client with `Execute`, `GetActionResult`, `UpdateActionResult`, `FindMissingBlobs`, and `BatchUpdateBlobs` in `fish-remote-cache/src/reapi.rs`.)*
- [ ] **eBPF File Tracing**: Kernel-level accurate input/output file capture on Linux. *(Partially implemented: the event model and hermeticity analysis exist in `fish-sandbox/src/file_events.rs`, but that module's own documentation states that attaching an automatic capture source — eBPF tracepoints, strace, platform APIs — "is deliberately out of scope until a real implementation lands". No kernel attachment exists today; events must be pushed manually via `FileEventRecorder::record_access`.)*

### 3. Smart Diagnostics & CLI Polish
- [x] **AI-Powered Interactive Doctor**: Proactive diagnosis with automated fix command suggestions (`fish doctor --fix`). *(`--fix` performs real remediation — schema-correct `fish.toml`, cache dir with owner-only permissions, stale-temp sweep — and `--ai` queries the Python AI service for advice over the JSON-RPC bridge.)*
- [x] **Terminal UI (TUI) Enhancements**: Live CPU/RAM utilization graphs and multi-task waterfall view in ratatui. *(Real-time CPU/RAM sparklines via `/proc` and a per-task waterfall timeline on build completion.)*

> **v0.3.x milestone completed (2026-08-21):** 7 of the 8 short-term Developer Experience & Protocol
> items are fully implemented; eBPF file tracing ships only its event-model and hermeticity-analysis
> foundation (automatic kernel capture is still open, tracked above).

---

## 🌟 Medium-term Goals (v0.4.x - v0.5.x) — Focus: Distributed Infrastructure & AI

### 1. Cloud-Native Distributed Infrastructure
- [x] **Kubernetes Operator (Go)**: Custom Resource Definitions (CRDs) for auto-scaling elastic worker fleets. *(`go/cmd/fish-k8s-operator` with the FishCluster CRD, controller, reconciler, and spot-node handling in `go/pkg/k8s`; shipped with v0.5.0 per CHANGELOG.)*
- [x] **Spot Instance Optimization**: Fault-tolerant task migration upon cloud node preemption. *(Rust-side `PreemptionRetryExecutor` in `fish-scheduler/src/preemption.rs` retries infrastructure-shaped failures then migrates to an on-demand fallback; spot-node logic in `go/pkg/k8s/spot.go`.)*
- [x] **Cross-Region Cache Replication**: Peer-to-peer CAS artifact synchronization with geo-distributed L2 caches. *(Region-aware `ReplicationTopology` with catalog tracking and TTL eviction in `fish-remote-cache/src/replication.rs`.)*

### 2. Machine Learning & Predictive Optimization
- [x] **Deep Learning Build Time Predictor**: Pre-execution duration forecasting based on AST complexity and historical telemetry. *(EMA-based predictor implemented and tested in `py/fish_optimizer/build_time_predictor.py`.)*
- [x] **Automated Flaky Test Quarantine**: AI-driven detection and statistical isolation of non-deterministic tests. *(Statistical flip detection in `py/fish_recommender/flaky_quarantine.py` plus the Rust `fish-flaky-detection` crate.)*
- [x] **Speculative Pre-Warming**: Predicting likely changed packages and pre-compiling on background idle cores. *(Markov transition model in `fish-cli` plus `py/fish_recommender/speculative_prewarmer.py`, whose transitive impact propagation was fixed.)*

### 3. Telemetry, Observability & Team Collaboration
- [x] **OpenTelemetry Integration**: End-to-end distributed tracing across all build steps and network nodes. *(OTLP/HTTP+JSON `OtlpExporter` honoring `OTEL_EXPORTER_OTLP_ENDPOINT` in `fish-analytics/src/otel_export.rs`; `fish build` exports root + per-task spans; cross-worker trace merging in the same crate.)*
- [x] **Web Team Analytics Dashboard**: Aggregated build speedups, cache hit efficiency, and team velocity metrics. *(JSONL persistence via `PersistentMetricsStore` and the `/api/team-stats` endpoint in `fish-dashboard/src/api.rs`.)*
- [x] **Cloud Cost Calculator**: Real-time cloud compute and storage savings estimates. *(`fish cost-estimate` with TOML pricing catalogs for AWS/GCP/Azure, LPT bin-packing, and spot/ondemand comparison.)*

### 4. Plugin Ecosystem
- [ ] **WebAssembly Plugin Engine**: Sandboxed Wasm plugins using Extism/WASI for custom toolchain adapters.
- [ ] **Plugin Marketplace Registry**: Decentralized plugin discovery and signed artifact distribution.

---

## 🏰 Long-term Vision (v1.0+) — Focus: Enterprise & Zero-Trust

### 1. Enterprise Security & Zero-Trust Execution
- [ ] **MicroVM Hardware Isolation**: Hermetic build execution inside ultra-lightweight Firecracker / Cloud-Hypervisor microVMs.
- [ ] **Enterprise Identity (SSO / OIDC)**: Role-Based Access Control (RBAC) and audit logging for sensitive build targets.
- [ ] **Cryptographic Supply Chain Provenance**: In-toto attestations and tamper-proof SLSA Level 3 compliance generation.

### 2. Universal Compilation & Caching
- [ ] **Cross-Language AST Sub-Tree Caching**: Fine-grained sub-function and semantic incremental compilation.
- [ ] **Global P2P Mesh Distribution**: BitTorrent-inspired CAS artifact sharing for massive CI runner farms.
- [ ] **Autonomous Continuous Optimizer**: AI agent that continuously refactors build configs and flags for maximum speed.

---

## 📅 Timeline Estimates

| Release | Focus Area | Target Horizon | Status |
| :--- | :--- | :--- | :--- |
| **v0.2.x** | Tri-Engine Core, 11 Backends, CAS, 5-Language Docs | Q3 2026 | ✅ Completed |
| **v0.3.x** | IDE Plugins, IPC Bridges, eBPF Tracing, LSP | Current | ✅ Completed |
| **v0.4.x - v0.5.x** | K8s Operator, Predictive ML, OpenTelemetry, Wasm | Q1 - Q2 2027 | 🟡 In Progress (infra, ML, and telemetry shipped; Wasm plugin engine + marketplace remain) |
| **v1.0** | MicroVM Sandboxing, Enterprise SSO, P2P Mesh, SLSA L3 | Q3 2027+ | ⚪ Vision |

---

## 💬 Feedback & Community Contributions

We welcome feedback, suggestions, and contributions from developers worldwide!
- Join discussions and feature requests via [GitHub Issues](https://github.com/requla11/fish/issues).
- Review our [Contributing Guide](contributing.md) and [Translation Guidelines](TRANSLATION.md).
