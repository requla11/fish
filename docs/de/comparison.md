# Comparison Matrix: Fish vs Other Build Systems

Fish is an ergonomic build-orchestration system written in Rust 2024 for modern polyglot monorepos. Here is a balanced, technical comparison with Bazel, Turborepo, and Buck2:

| Capability / Dimension | Fish | Turborepo | Bazel | Buck2 |
| :--- | :--- | :--- | :--- | :--- |
| **Primary Language** | Rust 2024 | Go / Rust | Java / C++ | Rust |
| **Language Support** | Polyglot (11+ native toolchains) | JS / TS focused | Polyglot (Starlark rules) | Polyglot (Starlark rules) |
| **Configuration Model** | Unified `fish.toml` / auto-detect | `turbo.json` | `WORKSPACE` + `BUILD.bazel` | `BUCK` files |
| **Setup Complexity** | Zero-config / Low | Low | High (fine-grained rules) | High (fine-grained rules) |
| **Hashing Engine** | Blake3 (parallel tree hashing) | SHA-256 | SHA-256 | Blake3 / SHA-256 |
| **CAS Compression** | Zstandard (Zstd L1-L22) + CoW | Tar.gz / Gzip | Zstd / Custom | Zstd / Custom |
| **Local Materialization** | Reflink / CoW (fallback to copy) | File copy | Symlinks / Hardlinks | Reflink / CoW |
| **Content Chunking** | FastCDC (16KB - 256KB block dedup) | Whole artifact archive | Whole artifact archive | Chunked CAS |
| **VFS Resolution** | In-Memory Snapshot Tree | FS scan | Inotify / Watchman daemon | Watchman / EdenFS |
| **Semantic Invalidation** | AST Interface Hash (.rmeta / ABI) | File hash only | Header-only compile | Header / rmeta compile |
| **AI Diagnostics** | Native IPC + Heuristics + Error Explainer | None | None | None |
| **Interactive Dashboard** | Built-in Web GUI + TUI | Vercel Web App | Third-party UI | Open-source console |

### Implementation evidence

Each Fish-specific capability above maps to real code in this repository:

| Capability | Where it lives |
| :--- | :--- |
| Blake3 hashing, Zstd compression, CoW/reflink materialization, FastCDC chunking | `crates/fish-cas/src/` (`reflink.rs`, `chunking.rs`) and `crates/fish-cache/src/` |
| In-memory VFS snapshot tree | `crates/fish-incremental/src/vfs.rs` (`VfsSnapshotTree`) — latency depends on tree size; see [benchmarks](benchmarks.md) |
| Semantic/ABI invalidation | `crates/fish-incremental/src/abi_extractor.rs` |
| AI diagnostics bridge | `crates/fish-cli/src/ai_bridge.rs` over the JSON-RPC daemon IPC |
| Web dashboard + TUI | `crates/fish-dashboard/` and `crates/fish-cli/src/tui.rs` |

Competitor columns describe those tools as publicly documented; we do not maintain their code.

---

## Detailed Architectural Breakdown

### Fish vs Turborepo
* **Language Scope:** Turborepo was created primarily for JavaScript/TypeScript monorepos. While it can run external task scripts, Fish natively discovers and coordinates 11+ toolchains (Cargo, Go modules, CMake, Python, Docker, etc.) directly from their native project manifests.
* **Storage Performance:** Turborepo uses standard archive tarballs and file copy. Fish uses Reflink / CoW filesystem extents and FastCDC content-defined chunking to minimize redundant disk I/O and network transfer.

### Fish vs Bazel
* **Design Philosophy & Trade-offs:** Bazel is engineered for massive codebases requiring strict, fine-grained hermetic sandboxing with detailed `BUILD.bazel` declarations for every target. Fish is designed as a lightweight, zero-config polyglot task orchestrator that discovers packages automatically, prioritizing quick developer onboarding over fine-grained action graph construction.
* **Runtime Architecture:** Bazel relies on a JVM daemon and dedicated sandbox wrappers. Fish runs as a single, self-contained native Rust binary with optional sandboxing and minimal resource overhead (~24 MB RAM vs 650+ MB for Bazel).

### Fish vs Buck2
* **Workflow & Usability:** Buck2 is a high-performance build tool designed for large-scale codebases utilizing Starlark rules and external filesystem watchers. Fish focuses on out-of-the-box polyglot workflow orchestration with built-in in-memory VFS, token-based GNU jobserver pool, and zero required build configuration.

---

## Empirical Case Study: Bazel vs Fish on `bazelbuild/examples`

> ⚠️ **Disclaimer — For Reference Only:**
> The empirical benchmarks and metrics presented in this case study were measured on a representative Windows x86_64 workstation (4 CPU cores, ~3.8 GB RAM) testing Google's official [`bazelbuild/examples`](https://github.com/bazelbuild/examples) repository at commit `3c479f4`.
> These figures are provided **strictly for illustrative reference and architectural understanding**. Actual production results will vary based on hardware specifications, disk I/O, network bandwidth (when downloading remote toolchain rules), and compiler configurations. Bazel provides hermetic sandboxing guarantees which require significant initialization overhead, whereas Fish prioritizes zero-config developer onboarding and low-latency native execution.

### Evaluation Setup

The test was conducted across all three stages of the Go tutorial (`stage1`, `stage2`, `stage3`) in `bazelbuild/examples`:
- **Clean Cache Procedure:**
  - **Bazel:** Executed `bazel clean --expunge` to completely wipe Bazel's output cache, sandboxes, and terminate the background JVM daemon.
  - **Fish:** Completely wiped `.fish/cache` and all local artifact directories (`build/`).
- **Target Scope:** Pure binary compilation without running test suites (`go_binary` for Bazel, and `go build` with `run_tests = false` for Fish).

### Measurement Results

| Module Tested | Target Name | Bazel 7.4.0 (Cold Build) | Bazel 7.4.0 (Warm Cached) | Fish 0.6.0 (Cold Build) | Fish 0.6.0 (Warm Cached) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Go Tutorial Stage 1** | `hello` | 165.53s | 23.55s | **1.08s** | **0.00092s (0.9ms)** |
| **Go Tutorial Stage 2** | `print_fortune` | 145.89s | 23.40s | **1.69s** | **0.00095s (0.9ms)** |
| **Go Tutorial Stage 3** | `fortune_test` | 149.68s | 23.70s | **0.99s** | **0.00088s (0.8ms)** |
| **Combined 3-Project Total** | **All 3 Targets** | **461.10s (~7.7 min)** | **~70.65s** | **3.76s** | **0.00275s (2.7ms)** |

### Architectural Analysis

1. **Cold Build Discrepancy (461.10s vs 3.76s):**
   - **Bazel:** Must bootstrap a Java Virtual Machine, download the Bazel 7.4 release, fetch `rules_go`, analyze 101 packages, configure over 10,800 targets, build an external Go SDK compiler helper (`builder.exe`), and compile the Go standard library in isolated sandbox layers.
   - **Fish:** Discovers standard toolchains installed on the host machine instantly (< 15ms startup), skips extraneous hermetic bootstrap downloads, and executes compiler commands directly into decentralized work-stealing job queues.

2. **Warm Cache Discrepancy (~70.65s vs 0.00275s):**
   - **Bazel:** Even when all actions are up-to-date, Bazel reconnects to the JVM server, performs Starlark evaluation, and reconciles thousands of target hashes.
   - **Fish:** Uses BLAKE3 tree-hashing to inspect file metadata and content fingerprints in microseconds. Because the source files are unchanged, Fish achieves a **100% cache hit rate** and exits in under 3 milliseconds across all three projects.
