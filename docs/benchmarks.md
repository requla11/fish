# Performance Benchmarks

Fish is engineered for ultra-low latency build orchestration and lockless parallelism.

## Benchmark Summary

> ⚠️ **Scope:** the table below is an *indicative, single-machine comparison* — it is a point-in-time
> measurement, not a certified result. It depends heavily on the sample project, hardware, toolchain
> versions, and warm-up state. Treat relative magnitudes, not exact numbers, as the signal.

| Build System | Cold Build (100 pkgs) | Warm Cached Build | Memory Footprint | Cross-Language Support |
| :--- | :--- | :--- | :--- | :--- |
| **Fish 0.5.0** | **18.4s** | **0.01s (Cache Hit)** | **~24 MB** | **11+ Languages Native** |
| Turborepo | 24.2s | 0.05s | ~85 MB | JS/TS Focused |
| Nx | 31.8s | 0.12s | ~180 MB | Monorepo JS/TS |
| Bazel | 22.1s | 0.04s | ~650 MB (JVM) | Multi-language |
| Cargo (Rust only)| 42.6s | 0.85s | ~120 MB | Rust only |

## Methodology & Reproducing

There are two distinct kinds of measurement, and only one of them is automated today:

### Internal primitive benchmarks (reproducible)

`cargo bench --workspace` runs criterion micro-benchmarks for Fish internals — storage and
caching primitives (CAS chunking/hashing/compression in `fish-cas`, cache lookup and eviction
in `fish-cache`) and scheduler graph ordering (`fish-scheduler`). These measure *Fish only*
and are directly reproducible:

```bash
cargo bench --workspace
```

### Cross-tool comparison (not yet automated)

The summary table compares end-to-end builds against Turborepo, Nx, Bazel, and Cargo. Running those
tools requires their own toolchains and a fixed sample project; there is **no committed script that
automates this comparison yet**, so the table cannot currently be regenerated from CI. If you repeat
the comparison, publish your sample-project definition, versions, and hardware alongside your numbers.

Automating a fair cross-tool benchmark harness is tracked as roadmap work; contributions welcome.
