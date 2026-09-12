# Fish Development (compatibility pointer)

> **Canonical document:** [`docs/development.md`](docs/development.md)
>
> This file exists for backwards compatibility — `README.md`, `AGENTS.md`,
> and older links point to `DEVELOPMENT.md` at the repo root. The full setup,
> test, lint, and benchmark instructions live in
> [`docs/development.md`](docs/development.md). Please read that file.

## Quick verify

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Note: `submodules/apple` and `submodules/banana` are private and excluded
from the workspace. Offline builds use `crates/fish-apple-shim` and
`crates/fish-banana-shim` automatically — no `git submodule update` needed.
