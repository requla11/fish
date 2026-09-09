# Design Document: Compiler Query Hooks (Moonshot)

## 1. Overview
The **Compiler Query Hooks** is a v2.0 Moonshot track that aims to replace coarse file-level cache invalidation with deep semantic incremental compilation. By integrating directly into compiler query systems (like `rustc`'s query engine or TypeScript's Compiler API), `fish` will execute builds based on fine-grained AST diffs rather than file timestamps or hashes.

## 2. Motivation
Currently, modifying a single comment or a private function in a 10,000-line Rust file causes `fish` to recompile the entire file (or crate). While `fish-incremental` implements a lightweight AST Sub-Tree Caching mechanism, it relies on regex and simple block matching (`extract_rust_functions`), which can be brittle and lacks type information.

Integrating directly with compiler APIs enables:
1. **Zero-cost whitespace/comment changes**: The compiler ignores them automatically.
2. **Type-aware invalidation**: Modifying a private struct field only invalidates functions that actually read/write that field.

## 3. Architecture

### 3.1 The Rustc Plugin / Wrapper
Instead of running `cargo build` or `rustc` as a black box, `fish` will spawn a customized `rustc` driver (using the `#![feature(rustc_private)]` API) or inject a dynamic library via `RUSTC_WRAPPER`. 

The wrapper intercepts the compiler's internal query system:
- It tracks which definitions (Functions, Structs, Traits) are modified.
- It queries the Dependency Graph (DepGraph) to determine exactly which downstream HIR (High-Level Intermediate Representation) nodes depend on the modified nodes.
- It bypasses compilation for unchanged nodes, fetching their pre-compiled object code or LLVM bitcode from the `fish-cas`.

### 3.2 TypeScript / Clang Hooks
- **TypeScript**: A Node.js daemon using the `typescript` npm package's Language Service API to emit only the changed AST subtrees.
- **Clang/C++**: Utilizing clang's `ASTMatcher` and module system to surgically extract and cache inline functions and template instantiations.

## 4. Implementation Plan (Prototype)

### Phase 1: Ejecting the File System
Replace the `blake3` file-hashing step in `fish-core` with an RPC call to a language-specific syntax server (e.g., `fish-ast-server`). 
The server parses the file into an AST and returns a structural hash (ignoring comments/whitespace).

### Phase 2: Compiler Internal State
Fork a minimal version of `rustc`'s driver to expose the `DepGraph`. 
Map `fish-cas` chunk hashes to `rustc`'s internal `Fingerprint` hashes, allowing `rustc` to load cached compilation phases (e.g., MIR optimization) directly from the remote cache mesh.

## 5. Security & Determinism
Semantic caching introduces risks: if the compiler hook misses a dependency edge, the build becomes non-deterministic or broken. 
We will mitigate this by continuously running a "shadow build" pipeline: 1% of semantic builds are dual-compiled from scratch to verify byte-for-byte equivalence, alerting on any discrepancies.
