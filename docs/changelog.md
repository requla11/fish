# Changelog & Version History

All notable changes to the Fish project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.6.0] - 2026-08-25

### Added
- **Cross-Language Protobuf Contracts**: Binary Google Protocol Buffers wire encoding and decoding across Rust, Go, and Python without heavy external compiler dependencies.
- **Wasm Plugin Engine & Security Audit**: Sandboxed WebAssembly plugins with capability auditing (`fish plugin audit`) and Ed25519 cryptographic signature verification.
- **Content-Addressable Storage (CAS) with ZSTD**: Ultra-fast BLAKE3 tree-hashing and multi-threaded Zstandard compression for deterministic L1/L2 build caching.
- **11 Polyglot Ecosystem Backends**: First-class zero-config support for Rust, Go, TypeScript/Node, Python, C/C++, Docker, Java, .NET, Swift, Dart, and Zig.
- **Adaptive Parallelism & Work-Stealing**: Chase-Lev work-stealing deque scheduler with execution-heuristic tail prioritization and RAM backpressure protection.

### Changed
- Refactored task graph dependency resolution to provide full cycle diagnostic traces instead of generic errors.
- Cache restoration now materializes declared task artifacts locally on cache hits.

## [v0.5.0] - 2026-08-24

### Added
- **5-Language Documentation Portal**: Full VitePress documentation localized in English, Vietnamese, Simplified Chinese, Traditional Chinese, and Japanese.
- **Distributed Coordinator (Go)**: High-concurrency worker cluster coordinator with heartbeat health tracking and HTTP/Protobuf dispatch.
- **AI Failure Analysis & Remediation (Python)**: Subprocess bridge for compiler error diagnostic parsing, root-cause identification, and predictive prewarming.

## [v0.3.0] - 2026-08-21

### Added
- **IDE Extensions**: Official VS Code Extension and Language Server Protocol (`fish lsp`) integration.
- **Interactive TUI Dashboard**: Real-time multi-threaded build progress, CPU/RAM utilization, and waterfall visualization.
- **eBPF Tracing**: Dynamic file access and dependency discovery on Linux systems.

## [v0.2.0] - 2026-08-10

### Added
- **Tri-Engine Core Architecture**: Pure Rust 2024 core orchestration engine coupled with auxiliary Go and Python services.
- **Fingerprint Cache Engine**: High-speed BLAKE3 hashing for task cache keys and change detection.
- **GNU Jobserver Pool**: Global concurrency governance preventing compiler thrashing.

## [v0.1.0] - 2026-08-01

### Added
- Initial experimental release of Fish build orchestrator with Rust and TypeScript backend support.
