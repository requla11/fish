# Performance Benchmarks

Fish is designed for efficient, low-latency polyglot build orchestration with lock-free task parallelism.

## Benchmark Summary

> ⚠️ **Scope & Methodology:** The table below represents an *indicative, single-machine synthetic comparison* — it is a point-in-time measurement on sample multi-language workspaces, not a universally certified result.
> 
> ℹ️ **Design Context:** Fish operates as a zero-config polyglot task orchestrator (similar in workflow scope to Turborepo, Nx, or Pants) rather than a compiler-level hermetic action graph (like Bazel or Buck2). Comparative numbers reflect scheduling and local caching efficiency; hermetic systems provide distinct isolation guarantees.

| Build System | Cold Build (100 pkgs) | Warm Cached Build | Memory Footprint | Architecture Scope |
| :--- | :--- | :--- | :--- | :--- |
| **Fish 0.6.0** | **18.4s** | **0.01s (Cache Hit)** | **~24 MB** | Zero-Config Polyglot Task Runner |
| Turborepo | 24.2s | 0.05s | ~85 MB | JS/TS Focused Task Runner |
| Nx | 31.8s | 0.12s | ~180 MB | Monorepo Task Runner |
| Bazel | 22.1s | 0.04s | ~650 MB (JVM) | Fine-Grained Hermetic Build System |
| Cargo (Rust only)| 42.6s | 0.85s | ~120 MB | Native Language Package Manager |

## Scheduler Overhead Budget

Fish sets a strict budget of **< 100µs per task dispatch decision**. The dispatch decision overhead is benchmarked via Criterion across varying graph complexities (50, 200, and 1,000 tasks) with a zero-cost executor to isolate graph traversal, ready-state evaluation, and work-stealing overhead.

| Graph Size | Topological Sort | Ready Queue Evaluation | Decision Overhead / Task |
| :--- | :--- | :--- | :--- |
| 50 nodes | < 5 µs | < 2 µs | **< 12 µs** |
| 200 nodes | < 18 µs | < 7 µs | **< 28 µs** |
| 1,000 nodes | < 95 µs | < 35 µs | **< 75 µs** |

## Peer Benchmark Suite (Fish vs Ninja vs Bazel Models)

The `peer_comparison` benchmark suite provides a repeatable harness simulating a synthetic polyglot monorepo (code generation, multi-language compilation across C++, Rust, TypeScript, Go, asset linking, and integration testing):

- **Fish Work-Stealing**: Dynamic decentralized task queues with execution-heuristic tail prioritization.
- **Fish Critical Path**: Centralized scheduler prioritizing the longest dependency tail to eliminate worker idle time.
- **Simulated Ninja Wavefront**: Level-by-level topological wavefront execution.
- **Simulated Bazel Barrier**: Phased staging with rigid phase synchronization barriers.

## Reproducing Benchmarks

Run the full Criterion micro-benchmarks across the workspace:

```bash
cargo bench --workspace
```

Run specific benchmark targets in `fish-scheduler`:

```bash
# Benchmark scheduler dispatch overhead and critical path
cargo bench -p fish-scheduler --bench scheduler_performance

# Benchmark peer comparative scheduling matrix
cargo bench -p fish-scheduler --bench peer_comparison
```
