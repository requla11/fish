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
* **Runtime Architecture:** Bazel relies on a JVM daemon and dedicated sandbox wrappers. Fish runs as a single, self-contained native Rust binary with optional sandboxing and minimal resource overhead.

### Fish vs Buck2
* **Workflow & Usability:** Buck2 is a high-performance build tool designed for large-scale codebases utilizing Starlark rules and external filesystem watchers. Fish focuses on out-of-the-box polyglot workflow orchestration with built-in in-memory VFS, token-based GNU jobserver pool, and zero required build configuration.
