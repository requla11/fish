# Polyglot Demo Project

> 🌐 **Translations & Contributions:** Want to translate or improve this document in your language? See our [Translation Guidelines](TRANSLATION.md).

A realistic contract-first monorepo demonstrating fish's multi-language build
capabilities — and especially its **automatic cross-language dependency
inference**: four projects in four toolchains are wired together by nothing
but the files they reference, with no `depends_on` declarations anywhere.

## Project Structure

```text
polyglot-demo/
├── py-worker/       # Python worker — OWNS the shared TaskEvent contract
│   └── contracts/   #     events.schema.json + topics.json (single source of truth)
├── go-service/      # Go HTTP API — reads the schema at runtime, tests it at build time
├── rust-service/    # Rust TCP service — embeds the schema at compile time (include_str!)
├── web-frontend/    # TypeScript frontend — imports topics.json directly
├── docker-compose/  # Docker configuration (builds after every service)
└── fish.toml        # fish configuration
```

## The cross-language story

The event contract lives in exactly one place: `py-worker/contracts/`. Every
other project references it through its own native mechanism:

| Project | Reference into py-worker | Coupling strength |
| :--- | :--- | :--- |
| `rust-service` | `include_str!("../../py-worker/contracts/events.schema.json")` | Compile error if contract moves |
| `go-service` | `main_test.go` reads `../py-worker/contracts/events.schema.json`; runtime default path | Test failure if contract breaks |
| `web-frontend` | `import { topics } from "../../py-worker/contracts/topics.json"` | Type-check failure if contract breaks |

Fish discovers these references while building and **infers the dependency
edges automatically** — py-worker is built before go-service, rust-service,
and web-frontend without anyone declaring it. You'll see it in the output:

```text
🔗 Inferring cross-language dependencies:
   ↳ web-frontend → py-worker (TypeScript project references `../../py-worker/...`)
🔗 Linked N cross-project task edge(s) from M inference(s) (disable with --no-infer-deps)
```

## Build with fish

From this directory:

```bash
# Build all services — note the inferred cross-project edges in the output
fish build

# Inspect the graph: py-worker sits upstream of its three consumers,
# docker-compose downstream of everything.
fish graph --format tree

# Run tests (go-service's test reaches into py-worker's contract directory)
fish test

# Prove the edges come from inference: same build, no linking
fish build --no-infer-deps -v
```

Each service is also runnable on its own toolchain:

```bash
(cd py-worker && python worker.py)      # stdlib only — validates events against the contract
(cd go-service && go test ./...)        # verifies the shared contract from the Go side
(cd rust-service && cargo run)          # fails to COMPILE if the contract file disappears
(cd web-frontend && npm install && npm run typecheck)  # type-checks against topics.json
```

## Language Support

- **Python** (`py-worker`): standard library only; owns the JSON Schema contract
- **Go** (`go-service`): Go modules, `net/http`
- **Rust** (`rust-service`): Cargo, tokio, serde_json
- **TypeScript** (`web-frontend`): npm + `tsc`, `resolveJsonModule`
- **Docker** (`docker-compose`): container image for the stack

## Features Demonstrated

- Multi-language discovery and parallel execution across four ecosystems
- Automatic cross-language dependency inference (contract-first pattern)
- Build-order correctness without any hand-written `depends_on`
- Cross-language caching and graph visualization
- Opt-out behavior via `--no-infer-deps`
