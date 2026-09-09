<div align="center">

<img src="docs/public/logo.png" alt="Fish Logo" width="180" />

# 🐟 Fish

**Das rasante, Cache-First Build-Orchestrierungssystem für polyglotte Monorepos**

[![CI](https://github.com/requla11/fish/actions/workflows/dogfood.yaml/badge.svg)](https://github.com/requla11/fish/actions/workflows/dogfood.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/requla11/fish)

[English](README.md) • [Tiếng Việt](README.vi.md) • [简体中文](README.zh-hans.md) • [繁體中文](README.zh-hant.md) • [日本語](README.ja.md)

</div>

---

**Fish** ist eine in **Rust 2024** entwickelte Hochleistungs-Build-Orchestrierungs-Engine. Es bietet die Geschwindigkeit und Einfachheit von Turborepo mit der polyglotten Leistungsfähigkeit von Bazel – **ohne komplexe Konfigurationssprachen wie Starlark oder benutzerdefinierte Build-DSLs zu erfordern**.

Fish erkennt automatisch Ihre Toolchains, analysiert Quellbäume, um sprachübergreifende Abhängigkeitskanten abzuleiten, plant Aufgaben über einen lock-free Work-Stealing-Pool und speichert jedes Artefakt mithilfe eines kryptografisch sicheren **BLAKE3** Content-Addressable Storage (CAS) und **Zstandard**-Komprimierung.

> 💡 **Hinweis:** Fish koordiniert bestehende Compiler und Paketmanager (Cargo, Go, npm/pnpm, Python, Clang usw.). Es ersetzt diese nicht. Es steht in keiner Verbindung zu [fish-shell](https://fishshell.com) – sie teilen nur den Namen.

---

## ✨ Wichtige Highlights

| Funktion | Beschreibung |
| :--- | :--- |
| ⚡ **Sub-Millisekunden-Scheduling** | Chase-Lev Work-Stealing-Queues und Critical-Path-Scheduling verteilen Aufgaben in <100µs. |
| 🌐 **11+ Sprach-Ökosysteme** | Native Backends für Rust, Go, TypeScript/JS, Python, C/C++, Java, .NET, Swift, Dart, Zig und Docker. |
| 🔗 **Automatische Abhängigkeitsableitung** | Contract-First sprachübergreifende Verknüpfung: Referenzen (wie `include_str!`, JSON-Importe) verdrahten DAG-Kanten automatisch ohne manuelles `depends_on`. |
| 💾 **High-Throughput CAS Cache** | Deduplizierter BLAKE3 Content-Addressable Storage mit mehrstufigem L1/L2-Caching und ZSTD-Komprimierung. |
| 📡 **Zero-Config P2P Cache** | Teilen Sie Build-Artefakte Peer-to-Peer über lokales WLAN / LAN mit Teamkollegen – null Cloud-Server-Kosten. |
| 🛡️ **Hermetische Isolierung** | Multi-Plattform-Sandboxing: Linux Namespaces & Landlock, macOS Seatbelt und Windows Security Tokens. |
| 📊 **Echtzeit Interaktive UI** | Integriertes Web-Dashboard (`fish ui`) mit einem interaktiven SVG-DAG-Visualisierer und Telemetrie-Graphen. |

---

## 🚀 Schnellinstallation

### 1-Zeilen-Installer

#### Linux & macOS
```bash
curl -fsSL https://raw.githubusercontent.com/requla11/fish/main/scripts/install.sh | sh
```

#### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/requla11/fish/main/scripts/install.ps1 | iex
```

---

### Paketmanager

| Plattform | Paketmanager | Befehl |
| :--- | :--- | :--- |
| **Windows** | **Scoop** | `scoop install https://raw.githubusercontent.com/requla11/fish/main/packaging/fish.json` |
| **Windows** | **Winget** | `winget install requla11.fish` |
| **macOS** | **Homebrew** | `brew tap requla11/fish https://github.com/requla11/homebrew-fish && brew install fish` |
| **Cargo** | **crates.io / Git** | `cargo install --git https://github.com/requla11/fish.git fish-cli` |

---

## 🏁 Schnellstart

Navigieren Sie zu einem beliebigen mehrsprachigen Repository und führen Sie aus:

```bash
# Baue den gesamten Workspace parallel mit intelligentem Caching
fish build

# Führe alle Testsuiten für jede Sprache aus
fish test

# Watch-Modus: Bei Dateiänderungen neu kompilieren und neu testen
fish dev

# Bereinige Build-Artefakte (oder alles einschließlich lokalem Cache mit --all)
fish clean --all

# Starte das interaktive Web-Dashboard & den DAG-Visualisierer
fish ui --open
```

### Probieren Sie die Polyglot-Demo aus

Wir fügen ein realistisches Contract-First Monorepo hinzu, das **Rust + Go + Python + TypeScript** kombiniert:

```bash
cd examples/polyglot-demo
fish build
fish graph --format tree
```

Ausgabe:
```text
🔗 Inferring cross-language dependencies:
   ↳ go-service → py-worker (Go project references `../py-worker/contracts/events.schema.json`)
   ↳ rust-service → py-worker (Rust project references `../../py-worker/contracts/events.schema.json`)
   ↳ web-frontend → py-worker (TypeScript project references `../../py-worker/contracts/topics.json`)
🔗 Linked 6 cross-project task edge(s) from 3 inference(s)

Build completed successfully.
  Tasks:     7 total (7 cached, 100% cache hit)
  Duration:  0.01s
```

---

## 🛠️ Unterstützte Ökosysteme

Fish erkennt und orchestriert nativ Projekte in 11 großen Ökosystemen:

| Ökosystem | Erkanntes Manifest | Standardaufgaben |
| :--- | :--- | :--- |
| **Rust** | `Cargo.toml` | `cargo check`, `cargo build`, `cargo test` |
| **TypeScript / Node** | `package.json`, `tsconfig.json` | `typecheck`, `build`, `test` |
| **Go** | `go.mod` | `go vet`, `go build`, `go test` |
| **Python** | `pyproject.toml`, `requirements.txt` | syntax compile, `pytest`, lint |
| **C / C++** | `CMakeLists.txt`, `fish.cc.json` | CMake configure, build, `ctest` |
| **Java** | `pom.xml`, `build.gradle` | compile, test |
| **.NET / C#** | `*.csproj`, `*.sln` | `dotnet build`, `dotnet test` |
| **Swift** | `Package.swift` | `swift build`, `swift test` |
| **Dart / Flutter** | `pubspec.yaml` | `dart analyze`, `dart test` |
| **Zig** | `build.zig` | `zig build`, `zig test` |
| **Docker / OCI** | `Dockerfile`, `docker-compose.yml` | Multi-stage image build, OCI compilation |

---

## 📋 Essenzielle Befehle

Fish hält seine CLI sauber, intuitiv und entwicklerfreundlich:

```text
Bauen & Testen:
  fish build             Baue alle im Projektgraphen erkannten Ziele
  fish check             Typ-Check und Validierung der Ziele ohne Verlinkung
  fish test              Führe alle Testsuiten im gesamten Workspace aus
  fish run [TARGET]      Baue und führe ein bestimmtes binäres Ziel aus
  fish dev (or watch)    Beobachte kontinuierlich Dateien und löse inkrementelle Rebuilds aus

Überprüfen & Verstehen:
  fish graph             Visualisiere den DAG als Stage-Bäume, DOT oder JSON
  fish why <QUERY>       Frage in natürlicher Sprache, warum ein Ziel neu gebaut wurde
  fish ui                Öffne das Echtzeit-Web-Dashboard & den interaktiven DAG-Visualisierer
  fish doctor            Diagnostiziere installierte Toolchains, Cache-Integrität und Umgebung

Warten & Bereinigen:
  fish clean             Entferne Projekt-Build-Ziele (mit -a/--all wird ~/.fish/cache gelöscht)
  fish fix               KI- & compiler-gestützte Fehlerdiagnose und Auto-Remediation
  fish ci init           Generiere optimierte CI/CD-Workflows (GitHub Actions, GitLab usw.)
  fish affected          Baue oder teste nur Pakete, die von Git-Änderungen betroffen sind
```

---

## 🏗️ Architektur & Workspace-Layout

Die Engine ist als modularer Rust-Workspace (28 Crates) strukturiert, der strikte Grenzisolation beibehält:

```text
crates/
  fish-core/         Workspace-Erkennung, Manifest-Modell und DAG-Merger
  fish-graph/        Abhängigkeitsgraph, topologische Sortierung und Abfrage-Algebra
  fish-executor/     Prozessausführung, Middleware-Kette und Response-Files
  fish-scheduler/    Paralleler Work-Stealing-Scheduler, GNU Jobserver-Pool, Racing und DTE
  fish-cache/        Fingerprint-Cache, zweiphasiges Pruning und morphische Hashes
  fish-cas/          Content-Addressable Artefaktspeicher mit BLAKE3 + ZSTD-Komprimierung
  fish-incremental/  Änderungserkennung, AST-Inferenz und Dirty-Rebuild-Explainer
  fish-backend-*/    11 Sprach- und Toolchain-Adapter, die EcosystemBackend implementieren
  fish-worker/       Verteilter Ausführungsserver und Streaming-VFS-Protokoll
  fish-remote-cache/ Durchsatzstarker Remote-Cache-Server mit Ed25519-Signatur-Gating
  fish-security/     Mehrschichtige Sicherheit, OSV-Schwachstellenscanner und SLSA-Herkunft
  fish-cli/          Einheitliche Kommandozeilenanwendung, Daemon-IPC und Terminal-Rendering
submodules/          Gekapselte Isolations-Engines (vendored):
  apple/             Hermetische Sandbox und OS-Prozessisolations-Daemon
  banana/            P2P-Swarm-Mesh, OCI-Container-Builder und Merkle-Ledger
examples/            Ausführbereite polyglotte Monorepo-Demonstrationen
```

---

## 🌿 Branch-Richtlinie

Fish folgt einem strikten Branch-Lebenszyklus:

```text
dev (aktive Entwicklung, Tests, Features)
  ↓
  ↓ verify: cargo test --workspace & cargo clippy
  ↓
main (stabile, produktionsbereite Releases)
```

- **`dev`** — Alle aktiven Arbeiten, Feature-Branches und Pull-Requests landen hier.
- **`main`** — Nur stabile getaggte Releases.

---

## 🧪 Entwicklung & Verifizierung

Um die Codebasis lokal zu überprüfen:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

---

## 📖 Dokumentation & Community

- [Architektur-Leitfaden](ARCHITECTURE.md) — Detailliertes Architekturdesign und Komponenten.
- [Entwicklungs-Setup](DEVELOPMENT.md) — Lokale Einrichtung, Debugging und Benchmarks.
- [Roadmap](ROADMAP.md) — Aktuelle Meilensteine, abgeschlossene Ziele und zukünftige "Moonshots".
- [Mitwirkungsrichtlinien](CONTRIBUTING.md) — Wie man Änderungen vorschlägt und Backends hinzufügt.
- [AI Agent Workflow](docs/AI_AGENT_WORKFLOW.md) — Best Practices für KI-Programmieragenten.

---

## 📄 Lizenz & Haftungsausschluss

Fish ist unter der [MIT-Lizenz](LICENSE) lizenziert.

> **Haftungsausschluss:** Dieses Projekt ist ein unabhängiges Build-Orchestrierungssystem. Andere nicht verbundene Werkzeuge, Pakete oder Projekte, die "fish" im Namen verwenden (wie `fish-shell`, `fish-image` usw.), sind unabhängig und werden nicht vom Fish Build-Orchestrierungsprojekt unterstützt, gesponsert oder befürwortet.
