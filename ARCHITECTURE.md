# Fish Architecture (compatibility pointer)

> **Canonical document:** [`docs/architecture.md`](docs/architecture.md)
>
> This file exists for backwards compatibility — `README.md`, `AGENTS.md`,
> and older links point to `ARCHITECTURE.md` at the repo root. To avoid
> content drift, the full architecture lives in one place:
> [`docs/architecture.md`](docs/architecture.md). Please read that file.

## Quick map

- Workspace discovery / manifest model: `crates/fish-core/`
- Dependency graph / DAG / queries: `crates/fish-graph/`
- Execution / middleware: `crates/fish-executor/`
- Scheduling / work-stealing / jobserver: `crates/fish-scheduler/`
- Fingerprint cache: `crates/fish-cache/`, CAS: `crates/fish-cas/`
- Language backends: `crates/fish-backend-*/` via `fish-backend-api::EcosystemBackend`
- CLI: `crates/fish-cli/`

## Offline shims

`submodules/apple` and `submodules/banana` are private companion repos and
are **not** required to build. The workspace builds offline via:

- `crates/fish-apple-shim/` (package `apple`) — minimal sandbox API
- `crates/fish-banana-shim/` (package `banana`) — minimal AST/OCI/P2P/ledger/telemetry API

To restore the full implementations once public, see `docs/architecture.md`
“Vendored submodules” section.
