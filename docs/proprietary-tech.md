# Fish Advanced Algorithms & Architectural Innovations

> 🌐 **Language Navigation / 多语言文档 / 多語言文檔 / ドキュメント言語:**
> [English](proprietary-tech.md) | [Tiếng Việt](vi/proprietary-tech.md) | [日本語](ja/proprietary-tech.md) | [简体中文](zh-hans/proprietary-tech.md) | [繁體中文](zh-hant/proprietary-tech.md)

---

## ⚡ Overview: Core Innovations in Fish

Fish integrates four specialized algorithms engineered to address scaling, cache invalidation, and incremental latency challenges in polyglot monorepos:

```
+-------------------------------------------------------------------------------+
|                      FISH CORE ALGORITHMIC INNOVATIONS                        |
+-------------------------------------------------------------------------------+
|  1. Poly-ABI Semantic HyperGraph (PASH)                                       |
|     --> Interface boundary symbol extraction & downstream invalidation cutoff |
|                                                                               |
|  2. Iso-Semantic Morphic Fingerprinting (IS-MFP)                              |
|     --> Dual-Key CAS normalization eliminating cross-environment cache misses |
|                                                                               |
|  3. Speculative Wavelet Pre-Execution (SWPE)                                  |
|     --> Edit-event energy classification & proactive dependency pre-warming   |
|                                                                               |
|  4. Virtual Binary Dispatch Table (CAS-VLink)                                 |
|     --> In-memory symbol dispatch overlay for rapid incremental iteration loops|
+-------------------------------------------------------------------------------+
```

---

## 🔬 1. Poly-ABI Semantic HyperGraph (PASH)
* **Location**: `crates/fish-graph`, `crates/fish-core`
* **Problem**: Traditional build systems invalidate all downstream targets whenever any upstream source file changes, even when public interfaces (APIs/signatures) remain unchanged.
* **Mechanism**:
  * Scans exported public interface signatures across all 11 supported backends (Rust, C/C++, Go, TS/JS, Python, Java, .NET, Swift, Dart, Zig, Docker).
  * Computes a deterministic `Symbolic Boundary Hash (SBH)`.
  * When private implementation details change while the `SBH` remains identical, PASH severs the invalidation cascade, saving redundant rebuilds.

---

## 🧬 2. Iso-Semantic Morphic Fingerprinting (IS-MFP)
* **Location**: `crates/fish-cache`, `crates/fish-cas`
* **Problem**: Workspace path variance, formatting differences, and environment entropy frequently cause 0% cache hit rates when switching between local workstations and CI runners.
* **Mechanism**:
  * Implements a **Dual-Key Hashing Architecture** (`ExactKey` + `MorphicKey`).
  * Normalizes workspace-relative paths (converting Windows backslashes to forward slashes) and filters volatile environment noise.
  * Falls back to morphic equivalence matching when exact hits miss, maximizing cache reuse across distinct execution environments.

---

## 🌊 3. Speculative Wavelet Pre-Execution (SWPE)
* **Location**: `crates/fish-incremental`
* **Problem**: Reactive build systems wait for manual save or build command execution, accumulating latency on every invocation.
* **Mechanism**:
  * Classifies editor event deltas into discrete energy levels (`TrivialWhitespace`, `CommentOnly`, `InternalStatement`, `GlobalInterface`).
  * Proactively prepares task dependency state and warm artifact buffers in background memory before full build execution.

---

## ⚡ 4. Virtual Binary Dispatch Table (CAS-VLink)
* **Location**: `crates/fish-executor`
* **Problem**: Full binary relinking creates significant overhead during rapid incremental iteration cycles.
* **Mechanism**:
  * Maintains an in-memory `VirtualBinaryDispatchTable` mapping symbol addresses and bytecode chunks.
  * Generates structured runtime binary overlays (`VLINK_DISPATCH_HEADER_V1`) for fast incremental symbol substitution and testing loops.
