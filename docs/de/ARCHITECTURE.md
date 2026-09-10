# Fish Architektur

> 🌐 **Übersetzungen & Beiträge:** Möchten Sie dieses Dokument in Ihre Sprache übersetzen oder verbessern? Siehe unsere [Übersetzungsrichtlinien](TRANSLATION.md).

Dieses Dokument beschreibt die High-Level-Architektur des Fish Build Orchestration Systems.

## Übersicht

Fish ist ein Cache-First, Polyglot Build Orchestration System, das für Monorepos und Polyglot-Projekte entwickelt wurde. Es verwendet einen Dependency Graph, einen parallelen Scheduler, einen Executor und einen CAS Artifact Cache, um die Build-Performance zu optimieren.

## Kernkomponenten

### 1. Workspace Discovery (`fish-core`)

**Zweck**: Erkennung und Modellierung der Projektstruktur

**Verantwortlichkeiten**:
- Scannen des Workspaces nach Packages/Projects
- Erkennen von Projekt-Typen basierend auf Manifest-Dateien
- Filtern von Eingabedateien durch Micro-Globs (`MicroInputFilter`)
- Erstellen des Dependency Graphs zwischen Packages
- Generieren von IDE Compilation Databases (`CompilationDatabase`, `compile_commands.json`)
- Verwalten und Isolieren von hermetischen Compiler-Toolchains (`ToolchainRegistry`, `ToolchainSpec`)
- Verwalten von Package-Metadaten

**Schlüsseltypen**:
- `Package`: Repräsentiert ein einzelnes Package/Projekt
- `Workspace`: Sammlung von Packages mit Dependencies
- `Manifest`: Projektkonfiguration (Cargo.toml, package.json, etc.)
- `MicroInputFilter`: Feingranularer Glob-Matcher und Datei-Filter
- `CompilationDatabase`: Standarddatenbank für Kompilierbefehle
- `ToolchainRegistry`: Manager für hermetische Toolchain-Konfigurationen

### 2. Build Graph (`fish-graph`)

**Zweck**: Modellierung von Build Dependencies, Execution Order und algebraischen Queries

**Verantwortlichkeiten**:
- Erstellen eines Directed Acyclic Graph (DAG) von Build-Tasks
- Berechnen des Topological Sort für die Execution Order
- Subgraph Merging für Polyglot-Monorepos (`merge_subgraph`)
- Dynamische Node-Erweiterung während der Laufzeit-Ausführung (`DynamicGraphExpander`)
- Verfolgen von Task-States (pending, running, completed, failed)
- Evaluierung algebraischer Queries (`GraphQueryEngine` unterstützt `deps()`, `rdeps()`, `allpaths()`, `somepath()`, `filter()`, `union()`, `intersect()`, `except()`)
- Erkennen von Circular Dependencies

**Schlüsseltypen**:
- `BuildGraph`: Gerichteter azyklischer Graph von Tasks
- `Node`: Einzelner Build-Task
- `NodeId`: Typsicherer Index in Graph-Strukturen
- `DynamicGraphExpander`: Dynamischer Generator für Sub-Tasks
- `GraphQueryEngine`: Evaluator für Graph-Query-Expressions
- `QueryExpr`: Algebraischer Query AST

### 3. Executor (`fish-executor`)

**Zweck**: Ausführen von Build Commands, Verwalten von Prozessen und Behandeln von File System Cloning

**Verantwortlichkeiten**:
- Spawnen und Verwalten von Build-Prozessen
- Erfassen von stdout/stderr
- Behandeln von Prozess-Timeouts und Cancellation
- Schnelles File System Cloning mittels Copy-on-Write-Extents und Hardlinks (`KernelCowCloner`)
- Schnelle Linker-Autoerkennung und Flag-Synthese (`LinkerDispatcher` unterstützt `mold`, `lld` und `msvc`)
- Automatische Synthese von Response-Dateien (`@fish_args.rsp`), wenn Argumente OS-Limits überschreiten
- Erweiterbare Task-Middleware-Pipeline (`TaskMiddleware`, `TurboLinker`, `SuperOptimizer`)
- Rückgabe von Execution Results

**Schlüsseltypen**:
- `CommandSpec`: Befehlsspezifikation mit Umgebung
- `AsyncExecutor`: Non-blocking Process Execution Engine
- `KernelCowCloner`: Copy-on-Write und Fast Cloner
- `LinkerDispatcher`: Moderner Linker-Detektor
- `ResponseFileWriter`: Argument-File-Synthesizer
- `TaskMiddleware`: Middleware Trait für Task Interception
- `ExecutionResult`: Ergebnis der Befehlsausführung

### 4. Scheduler (`fish-scheduler`)

**Zweck**: Scheduling von Tasks für parallele, spekulative und verteilte Ausführung

**Verantwortlichkeiten**:
- Aufrechterhalten der Ready Queue verfügbarer Tasks
- Verteilen von Tasks über verfügbare Worker
- Kernel Resource Governor (`KernelResourceGovernor`) zur Überwachung des System-Memory-Pressures und zur Drosselung der Concurrency
- Compiler Pipelining Coordination (`PipelinedCompilationCoordinator`), um nachgelagerte Kompilierungen bei Metadaten-Bereitschaft zu entsperren
- GNU Jobserver Pool Integration (`JobserverPool`) für globales Thread-Token-Management über Compiler hinweg
- Dynamisches Remote Racing (`DynamicRacingExecutor`): gleichzeitige lokale vs. remote Ausführung
- Distributed Task Execution (DTE) Bin-Packing (`DteBinPacker`) unter Verwendung von Longest Processing Time (LPT) Scheduling
- Echtzeit-Filesystem-Watcher-Daemon (`FsWatcherDaemon`) mit Dirty-Node-Invalidierung und Hot-Graph-Cache Pre-Warming
- Respektieren von Task Dependencies
- Behandeln von Task Completion und Failure

**Schlüsseltypen**:
- `Scheduler`: Task Scheduling Engine
- `KernelResourceGovernor`: Memory Pressure Monitor
- `PipelinedCompilationCoordinator`: Pipelined Stage Manager
- `JobserverPool`: Globaler token-basierter Concurrency Pool
- `DynamicRacingExecutor`: Lokaler vs. Remote Racer
- `DteBinPacker`: Balancierter Multi-Agent CI Partitioner
- `FsWatcherDaemon`: Echtzeit-Change-Listener und Dirty-Node-Tracker
- `WorkStealingPool`: Lock-freier Task Distributor

### 5. Cache (`fish-cache`)

**Zweck**: Fingerprint-basiertes Caching für inkrementelle Builds

**Verantwortlichkeiten**:
- Berechnen von File Content Fingerprints (Blake3)
- Cachen von Execution Results
- Bestimmen der Cache Validity
- Unterstützung der Cache Invalidation

**Schlüsseltypen**:
- `Fingerprint`: Content Hash mit Metadaten
- `CacheEntry`: Gecachtes Execution Result
- `FileLevelCache`: File-Level Caching Strategie

### 6. CAS Engine (`fish-cas`)

**Zweck**: Content-Addressable Storage für Artifact Caching

**Verantwortlichkeiten**:
- Speichern von Artifacts nach Content Hash
- Unterstützung für lokalen und Remote Storage
- Komprimieren von Artifacts (Zstandard)
- Bereitstellen von Deduplication

**Schlüsseltypen**:
- `ArtifactStore`: Artifact Storage Interface
- `LocalStorage`: Lokaler File System Storage
- `RemoteStorage`: Remote Storage (S3, GCS, MinIO)

### 7. Remote Cache (`fish-remote-cache`)

**Zweck**: Tiered L1/L2 Composite Caching

**Verantwortlichkeiten**:
- Lokaler L1-Cache für schnellen Zugriff
- Remote L2-Cache für Sharing
- Cache Population und Eviction
- Cache Hit/Miss Tracking

**Schlüsseltypen**:
- `CompositeCache`: Tiered Cache Implementierung
- `CachePolicy`: Cache Population und Eviction Policies

### 8. Worker (`fish-worker`)

**Zweck**: Distributed Build Execution

**Verantwortlichkeiten**:
- Remote Worker Discovery und Registrierung
- Task-Verteilung über Worker
- Result Collection und Aggregation
- Virtual File System für On-Demand File Access

**Schlüsseltypen**:
- `WorkerServer`: Worker-Daemon
- `ClusterExecutor`: Cluster Task Execution
- `VirtualFileSystem`: In-Memory VFS

### 9. Sandboxing (`fish-sandbox`)

**Zweck**: Hermetische Umgebungs-Isolation

**Verantwortlichkeiten**:
- Isolieren von Build Environments
- Kontrollieren von Filesystem Access
- Network Isolation
- Resource Limits

**Schlüsseltypen**:
- `Sandbox`: Sandbox Implementierung
- `SandboxConfig`: Sandbox Konfiguration

### 10. Plugin System (`fish-plugin`)

**Zweck**: Erweiterbares Rule System

**Verantwortlichkeiten**:
- Laden von Custom Plugins
- Script Plugin Execution (Shell, Python, Node, WASM, Lua)
- Plugin Discovery und Management
- Plugin API

**Schlüsseltypen**:
- `PluginManager`: Plugin-Manager
- `ScriptPlugin`: Script-basiertes Plugin
- `PluginExecutor`: Plugin Execution Engine

## Language Backends

Jedes Backend implementiert den einheitlichen `EcosystemBackend` Contract aus der `fish-backend-api` Crate und registriert sich selbst in `fish-cli/src/backend_registry.rs` — das Hinzufügen eines Ecosystems bedeutet, den Trait plus eine Registry-Zeile zu implementieren.

### Backend Interface

```rust
pub trait EcosystemBackend: Send + Sync {
    fn id(&self) -> &'static str;
    fn ecosystems(&self) -> &'static [Ecosystem];
    /// Cheap existence probe for this ecosystem's manifests.
    fn detect(&self, dir: &Path) -> bool;
    /// Config discovery + task-graph construction, owned per backend.
    fn build_task_graph(
        &self,
        dir: &Path,
        mode: BuildMode,
    ) -> Result<BuildGraph<Task>, String>;
}
```

### Supported Backends

- **Rust** (`fish-backend-rust`): Cargo Workspaces
- **C/C++** (`fish-backend-cc`): gcc/clang/msvc
- **Go** (`fish-backend-go`): go.mod
- **TypeScript/JS** (`fish-backend-ts`): package.json
- **Python** (`fish-backend-py`): pyproject.toml
- **Java** (`fish-backend-java`): Maven/Gradle
- **.NET** (`fish-backend-dotnet`): csproj/sln
- **Swift** (`fish-backend-swift`): Package.swift
- **Dart** (`fish-backend-dart`): pubspec.yaml
- **Zig** (`fish-backend-zig`): build.zig
- **Docker** (`fish-backend-docker`): Dockerfile

## Sicherheitsfunktionen

### 1. Artifact Signing (`fish-security` / `fish-remote-cache`)

- Ed25519 Signing via `FISH_SIGNING_SEED`; Public Key exportiert mit `fish signing-key`
- SLSA/in-toto Provenance Statements (`fish-security/src/slsa.rs`)
- Remote-Cache Signature Gate verifiziert jeden Download gegen `FISH_TRUSTED_KEYS` (`fish-remote-cache/signature_gate.rs`)
- Siehe `docs/signing.md` für den vollständigen Producer/Consumer-Flow

### 2. Security Scanner (`fish-security`)

- Dependency Vulnerability Scanning
- Multi-Backend-Unterstützung
- Severity-basiertes Blockieren
- CVSS Score Tracking

## CI/CD Generation

### CI Generator (`fish-ci-generator`)

Unterstützt mehrere CI/CD-Plattformen:
- GitHub Actions
- GitLab CI
- CircleCI
- Bitbucket Pipelines

### Matrix Generation

- Multi-Platform Support (Linux, macOS, Windows)
- Multi-Architecture (x86_64, ARM64)
- Version Matrices (Rust, Node, etc.)
- Dependency-basierte Optimierung

## Erweiterte Funktionen

### 1. Build Analytics (`fish-analytics`)

- Echtzeit Cache Hit Rate Tracking
- Build Metrics Collection
- Performance Visualisierung
- Optimierungsvorschläge

### 2. Incremental Analysis (`fish-incremental`)

- AST-basierte Dependency Inference (`DependencyInferenceEngine`) für Rust, TypeScript/JavaScript, Python und Go
- Dirty Rebuild Diagnostics (`DirtyExplainer`, `fish build --explain`) zur Identifizierung exakter Quellcode-Modifikationen oder Hash-Mismatches
- Build Pattern Detection und Hotspot Identifikation
- Refactoring-Vorschläge und Rebuild-Frequency-Analyse

### 3. Build Daemon & IPC (`fish-cli::daemon`)

- Background-Daemon (`FishDaemon`), der JSON-RPC 2.0 über durch Newlines getrennte Nachrichten spricht
- Transport: Unix Domain Socket auf Unix, TCP auf `127.0.0.1` auf Windows
- Port ist konfigurierbar: `fish daemon start --port <PORT>` (Standard `9527`)
- Warm Graph Caching für wiederholte Aufrufe
- Befehle: `fish daemon start`, `fish daemon status`, `fish daemon stop`

### 4. Profile-Guided Optimization (`fish-cli::pgo`)

- 2-Phasen LLVM PGO Workflow Orchestration (`PgoManager`)
- Automatisierte `-Cprofile-generate` Instrumentierung und `llvm-profdata merge`
- Rekompilierung mit `-Cprofile-use` für maximale Laufzeit-Performance

### 5. Task Pipeline Topology (`fish-cli::pipeline`)

- Topologische Task Pipelines im Turborepo/Nx-Stil, konfiguriert via `fish.toml`
- Cross-Package Dependency Rules (z. B. `^build`, um sicherzustellen, dass Dependency Outputs zuerst gebaut werden)
- Konfigurierbare Environment Variable und Input File Fingerprint Hashes

### Geplante Crates (noch nicht im Workspace)

Die folgenden Crates wurden in früheren Entwürfen beschrieben, existieren aber noch nicht im Workspace. Sie sind hier nur als Roadmap-Punkte aufgelistet:

- `fish-multiplatform` — Platform Detection, Target Triples, CI Matrices
- `fish-notifications` — Slack/Discord/E-Mail Build Notifications
- `fish-flaky-detection` — Statistische Flaky Test Erkennung und Retry-Policies
- `fish-docker-builder` — First-Class Docker Artifacts und Layer Caching (Docker-Orchestrierung lebt heute in `fish-backend-docker`)
- `fish-templates` — Teilbare Pipeline Templates (Handlebars Rendering)

## Vendored Submodules

Zwei Begleitprojekte sind als Git Submodules vendored und sind Mitglieder des Workspaces:

- **`submodules/apple`** — Hermetic Sandbox und Process Isolation Daemon (Kernel-Level Sandboxing, CoW Storage Jails, SLSA/SPDX/CycloneDX Provenance). Eigenständiges Projekt, nicht mit Apple Inc. verbunden.
- **`submodules/banana`** — Distribution, P2P Swarm und Supply-Chain Infrastructure Begleiter für Fish.

## Datenfluss

### Build Execution Flow

```
1. Workspace Discovery
   ↓
2. Dependency Graph Construction
   ↓
3. Cache Fingerprint Computation
   ↓
4. Scheduler Task Distribution
   ↓
5. Executor Process Management
   ↓
6. Result Collection & Caching
   ↓
7. Build Completion
```

### Distributed Build Flow

```
1. Worker Registration
   ↓
2. Task Distribution
   ↓
3. VFS File Streaming
   ↓
4. Remote Execution
   ↓
5. Result Aggregation
   ↓
6. Cache Population
```

## Performance-Optimierungen

### 1. Level Partitioning

Gruppiert unabhängige Packages pro Build-Level in einzelne Toolchain-Aufrufe, wodurch Process-Spawn-Overhead eliminiert wird.

### 2. Cache-First Execution

Fingerprint-basiertes Caching ermöglicht sofortige Rebuilds, wenn sich die Inputs nicht geändert haben.

### 3. Parallel Execution

Tasks werden parallel unter Berücksichtigung von Dependencies ausgeführt, um die CPU-Auslastung zu maximieren.

### 4. Incremental Builds

Es werden nur betroffene Packages basierend auf Dependency Graph Änderungen neu gebaut.

### 5. Distributed Execution

Remote-Worker ermöglichen horizontales Skalieren für große Projekte.

## Architektur-Status

Fish ist ein Single-Language Rust-Workspace. Es gibt keine Python- oder Go-Services in diesem Repository, und kein Crate verwendet derzeit gRPC/protobuf. Frühere Entwürfe dieses Dokuments beschrieben eine "Tri-Engine"-Architektur; diese Beschreibung passte nicht zur Codebasis und wurde entfernt.

### Aktueller Kern (implementiert)

- **`fish-core`**: Workspace Discovery, Manifest Models, feingranulares Input-Filtering.
- **`fish-graph`**: Dependency Graph, Topological Sort, algebraische Query-Evaluierung (`deps`, `rdeps`, `somepath`).
- **`fish-executor`**: Process Execution, Middleware Chain, Response File Generation.
- **`fish-scheduler`**: GNU Jobserver Pool, Work-Stealing, Parallel Execution.
- **`fish-cache`**: Multi-Tier Fingerprinting mit Blake3 und Two-Phase Pruning.
- **`fish-cas`**: Content-Addressable Artifact Storage mit ZSTD-Kompression.
- **`fish-cli`**: Terminal User Interface, betrieben durch ratatui und clap.

### Geplant: Cross-Language Contracts (`proto/`)

Die Dateien unter `proto/fish/v1/` (`build.proto`, `ai.proto`, `coordinator.proto`) sind zukunftsorientierte Schnittstellen-Entwürfe. Sie werden noch nicht von einem Crate kompiliert oder referenziert — der Workspace hat keine `prost`/`tonic` Dependencies. Heute ausgelieferte verteilte Funktionen verwenden stattdessen einfaches HTTP/JSON (siehe `crates/fish-worker` und `crates/fish-remote-cache`).

## Sicherheitsüberlegungen

- Kein `unsafe` Code in sicherheitskritischen Crates
- Input Validation über alle Backends hinweg
- Least Privilege für alle Operationen
- Audit Logging für Sicherheitsoperationen
- Sicheres Secret Management
- Ed25519 Artifact Signing und kryptographische SBOM-Generierung
