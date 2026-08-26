# Google Gemini & Antigravity Guide for Fish

## 🎯 Architecture & Invariants
- **Engine Core**: Rust 2024 Edition (MSRV 1.88+) across 28 crates (single-language Rust workspace; see `Cargo.toml`).
- **Go Services (`go/`)**: Optional Go 1.22+ auxiliary for distributed networking/worker gateways - not part of Rust workspace.
- **Python AI Engine (`py/`)**: Optional Python 3.11+ predictive models/AST remediation - not part of Rust workspace.
- **VS Code Extension (`vscode-extension/`)**: TypeScript client connecting to `fish lsp`.
- **Cross-language contracts (`proto/`)**: Forward-looking `build.proto`/`ai.proto`/`coordinator.proto` drafts only - not compiled, no `prost`/`tonic` dependency (see `ARCHITECTURE.md`).

## 🛠️ Mandatory Verification
```bash
# Rust Workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check

# Go Services
cd go && go test -v ./...

# Python AI Services
cd py && python -m unittest discover tests

# VS Code Extension
cd vscode-extension && npm run compile && npm run lint
```

## 🔒 Strict Engineering Rules
1. **No Fake Stubs / Fail-Loud**: Always implement real logic or return structured, typed errors. Never fake success.
2. **Cross-Platform Compatibility**: Always normalize cache keys and graph paths to forward slashes `/`. Ensure clean process termination with `kill_on_drop`.
3. **High Concurrency**: Leverage BLAKE3 hashing, ZSTD CAS storage, and Chase-Lev work-stealing queues.
