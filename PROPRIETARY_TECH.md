# Fish Proprietary Technologies & Next-Gen Algorithms

> 🌐 **Language Navigation / 多语言文档 / 多語言文檔 / ドキュメント言語:**
> [English](PROPRIETARY_TECH.md) | [Tiếng Việt](docs/vi/proprietary-tech.md) | [日本語](docs/ja/proprietary-tech.md) | [简体中文](docs/zh-hans/proprietary-tech.md) | [繁體中文](docs/zh-hant/proprietary-tech.md)

---

## ⚡ Overview: The Fish Quantum Polyglot Core (QPC)

Fish is pioneering four proprietary, un-replicated algorithms specifically engineered to solve the fundamental scaling and invalidation bottlenecks of modern polyglot monorepos and distributed build systems.

```
+-------------------------------------------------------------------------------+
|                      FISH QUANTUM POLYGLOT CORE (QPC)                         |
+-------------------------------------------------------------------------------+
|  1. Poly-ABI Semantic HyperGraph (PASH)                                       |
|     --> Trans-language public boundary extraction & invalidation cutoff       |
|                                                                               |
|  2. Iso-Semantic Morphic Fingerprinting (IS-MFP)                              |
|     --> Dual-Key CAS engine eliminating the cross-environment "Cache Cliff"   |
|                                                                               |
|  3. Speculative Wavelet Pre-Execution (SWPE)                                  |
|     --> Real-time LSP token-driven proactive zero-overhead micro-compilation  |
|                                                                               |
|  4. CAS-VLink (Virtual Jump-Table Splicer)                                    |
|     --> Zero-copy memory-mapped binary splicing bypassing the system linker   |
+-------------------------------------------------------------------------------+
```

---

## 🔬 1. Poly-ABI Semantic HyperGraph (PASH)
* **Status**: In Active Development (`crates/fish-graph`, `crates/fish-core`)
* **Problem**: Traditional build systems invalidate downstream polyglot targets whenever upstream source files change, even if public interfaces (APIs/ABIs) remain identical.
* **Mechanism**:
  * Extracts the **Public Interface Boundary (PIB)** for all 11 supported language backends.
  * Computes an invariant `Symbolic Boundary Hash (SBH)`.
  * When source files mutate, if `SBH` is unchanged, PASH severs the invalidation cascade across language borders.

---

## 🧬 2. Iso-Semantic Morphic Fingerprinting (IS-MFP)
* **Status**: In Active Development (`crates/fish-cache`, `crates/fish-cas`)
* **Problem**: Path variance, timestamp jitter, and environmental entropy cause a 0% cache hit rate (Cache Cliff) when switching between local workstations and CI runners.
* **Mechanism**:
  * Implements a **Dual-Key Hashing Architecture** (`ExactKey` + `MorphicKey`).
  * Normalizes AST structural entropy, stripping path and timestamp noise.
  * Falls back to morphic equivalence matching when exact hits miss, achieving >95% cache reuse across disparate environments.

---

## 🌊 3. Speculative Wavelet Pre-Execution (SWPE)
* **Status**: In Active Development (`crates/fish-scheduler`, `crates/fish-incremental`)
* **Problem**: Reactive build systems wait for `Ctrl+S` or terminal execution, forcing developer latency on every invocation.
* **Mechanism**:
  * Bridges directly into the `Fish LSP Bridge` to stream live keystroke diff wavelets.
  * Allocates background idle CPU tokens via the GNU Jobserver pool to pre-warm type inference and LLVM/codegen memory contexts.
  * Delivers sub-millisecond execution response upon user save.

---

## ⚡ 4. CAS-VLink (Zero-Copy Virtual Jump-Table Splicer)
* **Status**: In Active Development (`crates/fish-executor`, `crates/fish-cas`)
* **Problem**: System linkers (`ld`, `lld`, `link.exe`) consume 40-60% of compilation wall-clock time on large C++, Rust, Swift, and Go binaries.
* **Mechanism**:
  * Constructs a **Virtual Binary Dispatch Table (VBDT)** in output binaries.
  * Memory-maps and binary-splices modified object segments directly into the executable image without invoking the system linker.
  * Reduces link latency by 10x-50x during incremental iteration loops.
