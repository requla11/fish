<div align="center">

<img src="docs/public/logo.png" alt="Fish Logo" width="180" />

# 🐟 Fish

**The Blazing Fast, Cache-First Build Orchestration System for Polyglot Monorepos**

[![CI](https://github.com/requla11/fish/actions/workflows/dogfood.yaml/badge.svg)](https://github.com/requla11/fish/actions/workflows/dogfood.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/requla11/fish)

[English](README.md) • [Tiếng Việt](docs/vi/index.md) • [简体中文](docs/zh-hans/index.md) • [繁體中文](docs/zh-hant/index.md) • [日本語](docs/ja/index.md)

</div>

---

**Fish** is a high-performance build orchestration engine engineered in **Rust 2024**. It delivers the speed and simplicity of Turborepo with the polyglot power of Bazel — **without requiring complex configuration languages like Starlark or custom build DSLs**.

Fish automatically discovers your toolchains, analyzes source trees to infer cross-language dependency edges, schedules tasks across a lock-free work-stealing pool, and caches every artifact using cryptographically secure **BLAKE3** content-addressable storage (CAS) and **Zstandard** compression.

> 💡 **Notice:** Fish coordinates existing compilers and package managers (Cargo, Go, npm/pnpm, Python, Clang, etc.). It does not replace them. Unrelated to [fish-shell](https://fishshell.com) — they share only a name.

---

## ✨ Key Highlights

| Feature | Description |
| :--- | :--- |
| ⚡ **Sub-Millisecond Scheduling** | Chase-Lev work-stealing queues and critical-path scheduling dispatch tasks in <100µs. |
| 🌐 **11+ Language Ecosystems** | Native backends for Rust, Go, TypeScript/JS, Python, C/C++, Java, .NET, Swift, Dart, Zig, and Docker. |
| 🔗 **Automatic Dependency Inference** | Contract-first cross-language linking: references (like `include_str!`, JSON imports) automatically wire DAG edges without manual `depends_on`. |
| 💾 **High-Throughput CAS Cache** | Deduplicated BLAKE3 content-addressable storage with tiered L1/L2 caching and ZSTD compression. |
| 📡 **Zero-Config P2P Cache** | Share build artifacts peer-to-peer over local Wi-Fi / LAN with teammates — zero cloud server costs. |
| 🛡️ **Hermetic Isolation** | Multi-platform sandboxing: Linux namespaces & Landlock, macOS seatbelt, and Windows security tokens. |
| 📊 **Real-Time Interactive UI** | Built-in web dashboard (`fish ui`) featuring an interactive SVG DAG visualizer and telemetry graphs. |

---

## 🚀 Quick Install

### 1-Line Installer

#### Linux & macOS
```bash
curl -fsSL https://raw.githubusercontent.com/requla11/fish/main/scripts/install.sh | sh
```

#### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/requla11/fish/main/scripts/install.ps1 | iex
```

---

### Package Managers

| Platform | Package Manager | Command |
| :--- | :--- | :--- |
| **Windows** | **Scoop** | `scoop install https://raw.githubusercontent.com/requla11/fish/main/packaging/fish.json` |
| **Windows** | **Winget** | `winget install requla11.fish` |
| **macOS** | **Homebrew** | `brew tap requla11/fish https://github.com/requla11/homebrew-fish && brew install fish` |
| **Cargo** | **crates.io / Git** | `cargo install --git https://github.com/requla11/fish.git fish-cli` |

---

## 🏁 Quick Start

Navigate to any multi-language repository and run:

```bash
# Build the entire workspace in parallel with smart caching
fish build

# Run all test suites across every language
fish test

# Watch mode: re-compile and re-test on file changes
fish dev

# Clean build artifacts (or clean everything including local cache with --all)
fish clean --all

# Launch the interactive web dashboard & DAG visualizer
fish ui --open
```

### Try the Polyglot Demo

We include a realistic contract-first monorepo combining **Rust + Go + Python + TypeScript**:

```bash
cd examples/polyglot-demo
fish build
fish graph --format tree
```

Output:
```text
🔗 Inferring cross-language dependencies:
   ↳ go-service → py-worker (Go project references `../py-worker/contracts/events.schema.json`)
   ↳ rust-service → py-worker (Rust project references `../../py-worker/contracts/events.schema.json`)
   ↳ web-frontend → py-worker (TypeScript project references `../../py-worker/contracts/topics.json`)
🔗 Linked 6 cross-project task edge(s) from 3 inference(s)

Build completed successfully.
  Tasks:     7 total (7 cached, 100% cache hit)
  Duration:  0.01s
```

---

## 🛠️ Supported Ecosystems

Fish natively detects and orchestrates projects across 11 major ecosystems:

| Ecosystem | Manifest Detected | Default Tasks |
| :--- | :--- | :--- |
| **Rust** | `Cargo.toml` | `cargo check`, `cargo build`, `cargo test` |
| **TypeScript / Node** | `package.json`, `tsconfig.json` | `typecheck`, `build`, `test` |
| **Go** | `go.mod` | `go vet`, `go build`, `go test` |
| **Python** | `pyproject.toml`, `requirements.txt` | syntax compile, `pytest`, lint |
| **C / C++** | `CMakeLists.txt`, `fish.cc.json` | CMake configure, build, `ctest` |
| **Java** | `pom.xml`, `build.gradle` | compile, test |
| **.NET / C#** | `*.csproj`, `*.sln` | `dotnet build`, `dotnet test` |
| **Swift** | `Package.swift` | `swift build`, `swift test` |
| **Dart / Flutter** | `pubspec.yaml` | `dart analyze`, `dart test` |
| **Zig** | `build.zig` | `zig build`, `zig test` |
| **Docker / OCI** | `Dockerfile`, `docker-compose.yml` | Multi-stage image build, OCI compilation |

---

## 📋 Essential Commands

Fish keeps its CLI clean, intuitive, and developer-friendly:

```text
Build & Test:
  fish build             Build all targets discovered from the project graph
  fish check             Type-check and validate targets without linking
  fish test              Execute all test suites across the workspace
  fish run [TARGET]      Build and run a specific binary target
  fish dev (or watch)    Continuously watch files and trigger incremental rebuilds

Inspect & Understand:
  fish graph             Visualize the DAG as stage trees, DOT, or JSON
  fish why <QUERY>       Ask in natural language why a target was rebuilt
  fish ui                Open the real-time web dashboard & interactive DAG visualizer
  fish doctor            Diagnose installed toolchains, cache integrity, and environment

Maintain & Clean:
  fish clean             Remove project build targets (pass -a/--all to wipe ~/.fish/cache)
  fish fix               AI & compiler-grounded error diagnosis and auto-remediation
  fish ci init           Generate optimized CI/CD workflows (GitHub Actions, GitLab, etc.)
  fish affected          Build or test only packages affected by git changes
```

---

## 🏗️ Architecture & Workspace Layout

The engine is structured as a modular Rust workspace (28 crates) maintaining strict boundary isolation:

```text
crates/
  fish-core/         Workspace discovery, manifest model, and DAG merger
  fish-graph/        Dependency graph, topological sort, and query algebra
  fish-executor/     Process execution, middleware chain, and response files
  fish-scheduler/    Parallel work-stealing scheduler, GNU jobserver pool, racing, and DTE
  fish-cache/        Fingerprint cache, two-phase pruning, and morphic hashes
  fish-cas/          Content-addressable artifact storage with BLAKE3 + ZSTD compression
  fish-incremental/  Change detection, AST inference, and dirty rebuild explainer
  fish-backend-*/    11 language and toolchain adapters implementing EcosystemBackend
  fish-worker/       Distributed execution server and streaming VFS protocol
  fish-remote-cache/ High-throughput remote cache server with Ed25519 signature gating
  fish-security/     Multi-layer security, OSV vulnerability scanner, and SLSA provenance
  fish-cli/          Unified command-line application, daemon IPC, and terminal rendering
submodules/          Vendored companion isolation engines:
  apple/             Hermetic sandbox and OS process isolation daemon
  banana/            P2P swarm mesh, OCI container builder, and Merkle ledger
examples/            Ready-to-run polyglot monorepo demonstrations
```

---

## 🌿 Branch Policy

Fish follows a strict branch lifecycle:

```text
dev (active development, tests, features)
  ↓
  ↓ verify: cargo test --workspace & cargo clippy
  ↓
main (stable, production-ready releases)
```

- **`dev`** — All active work, feature branches, and pull requests land here.
- **`main`** — Stable tagged releases only.

---

## 🧪 Development & Verification

To verify the codebase locally:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

---

## 📖 Documentation & Community

- [Architecture Guide](ARCHITECTURE.md) — In-depth architectural design and components.
- [Development Setup](DEVELOPMENT.md) — Local setup, debugging, and benchmarks.
- [Roadmap](ROADMAP.md) — Current milestones, completed targets, and future moonshots.
- [Contributing Guidelines](CONTRIBUTING.md) — How to propose changes and add backends.
- [AI Agent Workflow](docs/AI_AGENT_WORKFLOW.md) — Best practices for AI coding agents.

---

## 📄 License & Disclaimer

Fish is licensed under the [MIT License](LICENSE).

> **Disclaimer:** This project is an independent build orchestration system. Other unrelated tools, packages, or projects using "fish" in their names (such as `fish-shell`, `fish-image`, etc.) are independent and not affiliated with, sponsored, or endorsed by the Fish build orchestration project.
