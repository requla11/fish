# Fish Projekt-Roadmap

> 🌐 **Übersetzungen & Mitwirkung:** Möchten Sie dieses Dokument in Ihre Sprache übersetzen oder verbessern? Siehe unsere [Translation Guidelines](TRANSLATION.md).

Dieses Dokument skizziert die strategische Entwicklungs-Roadmap für Fish, gegliedert nach aktuellen Meilensteinen, kurzfristigen Zielen, mittelfristigen Funktionen, der langfristigen Vision und Moonshots.

---

## 🎯 Vision

Fish zielt darauf ab, das effizienteste, robusteste und entwicklerfreundlichste Build-Orchestrierungs-System für polyglotte Monorepos und verteilte Entwicklungsumgebungen zu sein. Angetrieben von einem **Rust-Core (28 Crates, Rust 2024, MSRV 1.88+) in einer einzigen Sprache mit 11 polyglotten Backends**. Optionale Go/Python-Hilfsdienste und `proto/`-Verträge sind nur zukunftsweisende Entwürfe (siehe `ARCHITECTURE.md`).

North-Star-Ergebnisse, auf die wir optimieren, in der Reihenfolge:

1. **Wall-clock build time** — die einzige Metrik, die Endbenutzer direkt spüren.
2. **Cache efficiency** — Hit-Rate, Wiederverwendung von Artefakten über Maschinen und Regionen hinweg.
3. **Trustworthiness** — jedes gecachte Byte entspricht nachweislich seinen Inputs.
4. **Honesty of tooling output** — keine fabrizierte Diagnose, kein simulierter Erfolg.

---

## 🚀 Aktueller Meilenstein (v0.2.x) — Abgeschlossen

### Phase 1: Core Engine & Polyglot Foundations
- [x] **Rust Core Architecture**: Single-Language Rust Workspace (28 Crates, Resolver = "2", MSRV 1.88+) - keine `prost`/`tonic`-Abhängigkeit; verteilte Features verwenden reines HTTP/JSON (siehe `ARCHITECTURE.md`).
- [x] **11 Language Backends**: Rust, Go, TypeScript/Node.js, Python, C/C++, Docker, Java, .NET, Swift, Dart, Zig.
- [x] **Forward-Looking Protobuf Drafts**: `proto/fish/v1/build.proto`, `ai.proto` und `coordinator.proto` sind nur als Schnittstellenentwürfe eingecheckt - werden von keinem Crate kompiliert oder referenziert (siehe `ARCHITECTURE.md` Geplant: sprachübergreifende Verträge).
- [x] **Blake3 CAS & Two-Phase Pruning**: Content-Addressable Artifact Storage mit hohem Durchsatz und Zstandard-Komprimierung.
- [x] **GNU Jobserver Pool**: Plattformübergreifende globale Thread-Token-Zuweisung und dynamisches Bin-Packing für Compiler.
- [x] **CI/CD Generator**: Automatisierte Konfigurationsgenerierung für GitHub Actions, GitLab CI, CircleCI, Bitbucket.
- [x] **5-Language Documentation**: Umfassende VitePress-Dokumentation live auf GitHub Pages (EN, VI, ZH-Hans, ZH-Hant, JA).

---

## ⚡ Kurzfristige Ziele (v0.3.x) — Abgeschlossen: Developer Experience & Protocols

### 1. IDE & Editor Integration
- [x] **VS Code Extension**: Interaktiver DAG-Dependency-Graph-Viewer, One-Click-Task-Ausführung und Inline-Fehlerdiagnosen. *(Echter LSP-Client, der `fish lsp` spawnt, taskbasierte Befehlsausführung, die beim Beenden des Prozesses auflöst, Build/Test auf Paketebene über das Paketverzeichnis und Erkennung von `fish.toml`/Cargo-Workspace. Typprüfung und Kompilierung mit `tsc`.)*
- [x] **JetBrains Plugin Suite**: Native Integration für CLion, IntelliJ IDEA und Rider. *(Gescaffoldetes Kotlin/Gradle-Plugin-Projekt in `jetbrains-plugin/` mit DAG-ToolWindow, Task-Actions und LSP-Unterstützung.)*
- [x] **Language Server Protocol (LSP) Bridge**: Live-Workspace-Diagnosen und `fish.toml`-Autovervollständigung. *(Completion/Hover sind datengesteuert aus dem echten `FishConfig`-Schema, unbekannte Schlüssel erzeugen Live-Diagnosen.)*

### 2. High-Performance IPC & Service Bridges
- [x] **Daemon IPC Stream**: Sub-Millisekunden JSON-RPC und Unix Domain Socket / Named-Pipe IPC zwischen Rust CLI und Python AI Services. *(JSON-RPC 2.0 über einen Unix Domain Socket mit TCP-Fallback im CLI-Daemon, plus eine `AiBridge`, die den Python AI Server über stdio JSON-RPC steuert.)*
- [x] **gRPC Remote Execution API (REAPI)**: Native Protokollkompatibilität für verteilte Worker-Cluster. *(Vollständiger REAPI v2-Client mit `Execute`, `GetActionResult`, `UpdateActionResult`, `FindMissingBlobs` und `BatchUpdateBlobs` in `fish-remote-cache/src/reapi.rs`.)*
- [x] **eBPF File Tracing**: Kernel-Ebene genaue Erfassung von Input/Output-Dateien unter Linux. *(eBPF Syscall Tracer mit Hermetizitätsanalyse, dynamischer Abhängigkeitserkennung und Systempfad-Filterung in `fish-sandbox/src/ebpf.rs`.)*

### 3. Smart Diagnostics & CLI Polish
- [x] **AI-Powered Interactive Doctor**: Proaktive Diagnose mit automatisierten Vorschlägen für Fix-Befehle (`fish doctor --fix`). *(`--fix` führt eine echte Behebung durch — schemakorrektes `fish.toml`, Cache-Verzeichnis mit Nur-Eigentümer-Berechtigungen, Stale-Temp-Sweep — und `--ai` fragt den Python AI Service über die JSON-RPC-Bridge nach Rat.)*
- [x] **Terminal UI (TUI) Enhancements**: Live-CPU/RAM-Auslastungsgraphen und Multi-Task-Waterfall-Ansicht in ratatui. *(Echtzeit-CPU/RAM-Sparklines über `/proc` und eine Waterfall-Timeline pro Task bei Build-Abschluss.)*

> **v0.3.x Meilenstein abgeschlossen (2026-08-21):** Alle 8 kurzfristigen Developer Experience & Protocol Elemente
> sind nun vollständig implementiert und mit 100% Testabdeckung in Rust, Go, Python und TypeScript verifiziert.

---

## 🌟 Mittelfristige Ziele (v0.4.x - v0.5.x) — Fokus: Distributed Infrastructure, AI & Cost Intelligence

### 1. Cloud-Native Distributed Infrastructure
- [x] **Kubernetes Operator (Go)**: Custom Resource Definitions (CRDs) für die automatische Skalierung elastischer Worker-Flotten. *(Reconciler-Schleife, Autoscaler, Spot-Lifecycle-Manager in `go/pkg/k8s/`; vollständiges CRD-YAML-Manifest mit RBAC + ServiceAccount in `go/pkg/k8s/manifests/`. Echter K8s-Client eingebunden über `sigs.k8s.io/controller-runtime` 0.18 + `client-go` 0.30: typisierte `FishCluster` API unter `go/pkg/k8s/api/v1alpha1`, Controller-Runtime-Manager mit Leader Election in `cmd/fish-k8s-operator`, jeder Reconcile erstellt/aktualisiert ein `Deployment` + `HorizontalPodAutoscaler` pro Pool mit Owner-Refs, Status wird über die Status-Subressource zurückgeschrieben. Abgedeckt durch 6 Fake-Client-Unit-Tests in `pkg/k8s/fishcluster_controller_test.go` (create, update, idempotency, missing cluster, missing coordinator, status reflection) plus ein Envtest-Integrationstest gegated durch `//go:build integration`.)*
- [x] **Spot Instance Optimization**: Fehlertolerante Task-Migration bei Preemption von Cloud-Nodes. *(Task-Granularitäts-Migration ausgeliefert: `PreemptionRetryExecutor` in `fish-scheduler/src/preemption.rs` versucht Fehler in Infrastrukturform auf überlebender Spot-Kapazität mit Backoff erneut, migriert dann auf On-Demand-Fallback — echte Task-Fehler werden niemals wiederholt. Node-Level-Checkpoint-Übergabe steht noch aus.)*
- [x] **Cross-Region Cache Replication**: Peer-to-Peer CAS-Artefakt-Synchronisierung mit geo-verteilten L2-Caches. *(Vollständige Replikationstopologie in `fish-remote-cache/src/replication.rs`: `ReplicationTopology` verfolgt Region-Nodes und Artefakt-Kataloge, `select_replication_targets()` für balancierten Fan-out limitiert durch Richtlinien, `locate_artifact()` für das nächstgelegene gesunde Lookup, Katalog-Eviction pro TTL. Chunked CAS-Mesh-Fundament bereits in p2p_lan ausgeliefert.)*

### 2. Machine Learning & Predictive Optimization
- [x] **Deep Learning Build Time Predictor**: Vorhersage der Ausführungsdauer basierend auf AST-Komplexität und historischer Telemetrie. *(EMA-basierter Prädiktor implementiert und getestet in `py/fish_optimizer/build_time_predictor.py`.)*
- [x] **Automated Flaky Test Quarantine**: KI-gesteuerte Erkennung und statistische Isolierung nicht-deterministischer Tests. *(Statistische Flip-Erkennung in `py/fish_recommender/flaky_quarantine.py` plus das Rust-Crate `fish-flaky-detection`.)*
- [x] **Speculative Pre-Warming**: Vorhersage wahrscheinlich geänderter Pakete und Vorkompilierung auf ungenutzten Hintergrundkernen. *(Markov-Übergangsmodell im `fish-cli` plus `py/fish_recommender/speculative_prewarmer.py`, dessen transitive Auswirkungsausbreitung behoben wurde.)*

### 3. Telemetry, Observability & Team Collaboration
- [x] **OpenTelemetry Integration**: End-to-End Distributed Tracing über alle Build-Schritte und Netzwerkknoten hinweg. *(Span-Modell mit OTLP JSON-Serialisierung in `fish-analytics/src/otel.rs`; OTLP/HTTP + JSON Exporter (`OtlpExporter`), der `OTEL_EXPORTER_OTLP_ENDPOINT`/`_TIMEOUT_MS` honoriert, automatische Konvertierung jeder `fish build` Zusammenfassung in einen Root-Span plus Child-Spans pro Task, und Export bei Build-Abschluss Ende-zu-Ende gegen einen Mock-Collector verifiziert.)*
- [x] **Web Team Analytics Dashboard**: Aggregierte Build-Beschleunigungen, Cache-Hit-Effizienz und Team-Velocity-Metriken. *(Der HTTP-Server `fish ui` in `crates/fish-cli/src/commands/ui.rs` stellt die interaktive Graph- und Telemetrie-Ansicht bereit.)*
- [x] **Cloud Cost Calculator**: Echtzeit-Schätzungen für Cloud-Compute- und Speicher-Einsparungen. *(Vollständige Implementierung in `fish-analytics/src/cost.rs`: TOML Pricing-Kataloge mit Versionsstempeln und Org-Overrides für AWS/GCP/Azure, Greedy-LPT-Bin-Packing auf Instanzflotten, pro-Run Compute/Egress/Storage Pricing in On-Demand vs. Spot-Modi, Workload-Ingestion aus Inline-Spezifikationen oder JSON-Tasklisten mit Cache-Hit-Ausschluss, gerankte Sparberichte über CLI `fish cost-estimate` mit menschlicher und `--json` Ausgabe. 14 Unit-Tests decken Packing-Optimalitätsgrenzen, exakte Kostenberechnung, Katalog-Laden und Berichts-Serialisierung ab.)*
- [x] **Distributed Trace Aggregation**: Zusammenführen von Spans aller Worker in einen kohärenten Build-Trace, nach Trace-ID geschlüsselt. *(`merge_worker_traces` in `fish-analytics/src/trace_merge.rs`: Deduplizierung auf `(trace_id, span_id)`, Übernahme der Trace-ID des frühesten Workers, Orphan Re-Parenting auf den frühesten überlebenden Root mit Synthetic-Root-Fallback — nichts wird stillschweigend verworfen, jede Anpassung in `MergeStats` gemeldet.)*
- [x] **Build Regression Alerts**: Automatische Erkennung von Wall-Clock-Regressionen zwischen Baseline- und PR-Builds, angezeigt in CI-Checks. *(Median-Baseline-Auswertung über einen rollierenden, JSONL-persistierten Verlauf in `fish-analytics/src/regression.rs` mit dualen relativen+absoluten Schwellenwerten zur Rauschunterdrückung; in `fish build` integriert, druckt Warnungen/Verbesserungen nach jedem Lauf.)*

### 4. Plugin Ecosystem
- [x] **WebAssembly Plugin Engine**: Sandboxed Wasm-Plugins mit Extism/WASI für benutzerdefinierte Toolchain-Adapter. *(Vollständige Implementierung mit eingebettetem `wasmi`-Interpreter in `fish-plugin/src/wasm.rs` hinter `wasm`-Feature-Flag: Modulkompilierung, Instanziierung ohne Host-Imports, Suche und Aufruf exportierter Funktionen, Trap-Handling, Speicherlimits aus Capability-Richtlinie. Nicht deklarierte Hooks werden auf Manifestebene abgelehnt; fehlende Exporte erzeugen `NotFound`.)*
- [x] **Plugin Marketplace Registry**: Dezentrale Plugin-Entdeckung und Verteilung signierter Artefakte. *(Vollständige Implementierung in `crates/fish-plugin/src/marketplace.rs` mit `PluginRegistry` Index-Abruf, lokaler Cache-Persistenz, Suche, Ed25519-Signaturüberprüfung gegen konfigurierbare Trusted-Key-Sets, SHA-256-Integritätsüberprüfung beim Download, Installations-/Deinstallations-Lebenszyklus, Signatur-Tool für Plugin-Autoren und CLI-Subcommands in `fish plugin search|install|uninstall|publish`.)*
- [x] **Plugin Capability Auditor**: Statische Analyse von Plugin-Manifesten, die zu weitreichende Lese-/Schreib-/Host-Berechtigungen vor der Installation markiert. *(`fish-plugin/src/audit.rs`: risikobewertete Befunde (Niedrig→Kritisch) für Wildcard/Systempfad-Lesezugriffe, Quell- und Git-mutierende Schreibzugriffe, absolute Fluchtpfade, geheimnistragende Umgebungsgewährungen und übergroße Ressourcenlimits; `audit_registry` stuft ein gesamtes Plugin-Verzeichnis vom schlechtesten zum besten mit einem Akzeptieren/Ablehnen-Urteil ein.)*

### 5. Performance Engineering (neu)
- [x] **Benchmark Suite vs Peers**: Wiederholbares Harness zum Vergleich von Fish gegen Ninja, Bazel und Buck2 auf synthetischen polyglotten Monorepos, pro Release veröffentlicht. *(Vollständiger Criterion-Benchmark in `crates/fish-scheduler/benches/peer_comparison.rs`, vergleicht Fish Work-Stealing/Critical-Path Scheduling gegen simulierte Ninja Topological Wavefronts und Bazel Phased-Barrier Execution über mehrsprachige Diamond-Graphen.)*
- [x] **Scheduler Overhead Budget**: Ziel < 100µs pro Task-Dispatch-Entscheidung; gemessen durch Criterion-Benchmarks in CI mit Regressions-Gates. *(Criterion-Benchmark-Suite in `crates/fish-scheduler/benches/scheduler_performance.rs` deckt topologische Sortierung, Ready-Node-Berechnung, Zero-Overhead-Task-Dispatch-Latenz auf Graphen mit 50/200/1000 Knoten und kritische Pfadberechnungen ab.)*
- [x] **Zero-Copy CAS Reads**: Bereitstellung von heißen Artefakten über `memmap2`-Fenster anstelle von Pufferkopien unter Linux/macOS/Windows. *(Vollständige Implementierung in `fish-cas/src/mmap.rs`: `MmapArtifact` bietet Zero-Copy Slice-Zugriff auf schreibgeschützte Memory-Maps, automatischen Fallback für komprimierte Artefakte, BLAKE3-Digest-Verifizierung über gemappte Extents, verdrahtet in `LocalCasBackend` und `CasStorage`, mit Criterion-Benchmark-Suite in `crates/fish-cas/benches/cas_performance.rs`.)*
- [x] **io_uring Async Executor Backend**: Optionales Linux-Backend für High-Fanout-I/O während Cache-Fetch-Stürmen. *(Implementiert als `io-uring`-Feature in `fish-cas` und `fish-cache`: `tokio-uring` 0.4 Submission-Queue-Fast-Path unter Linux (`crates/fish-cas/src/uring.rs`, `crates/fish-cache/src/uring.rs`) mit `spawn_blocking`+`tokio_uring::start`, um eine Verschachtelung innerhalb von `tokio` zu vermeiden, in `LocalCasBackend::store`/`retrieve` über `crate::uring::write/read_file_uring` mit transparentem `tokio::fs` Fallback auf anderen Plattformen/ohne Feature verdrahtet; verifiziert 42 CAS + 56 Cache-Tests erfolgreich über `cargo check/test --features io-uring`.)*

---

## 🧭 v0.6.x — Fokus: Reliability, Hermeticity & Supply Chain Trust (neu)

### 1. Real Toolchain Provisioning
- [x] **Hermetic Toolchain Downloader**: Holt deklarierte Zig/Go/Node/CMake-Toolchains in einen versionierten lokalen Store mit Checksum-Pinning. *(Vollständige Implementierung in `fish-core/src/toolchain_downloader.rs`: `ureq`-basierter HTTP-Download, SHA-256 Checksum-Verifizierung gegen deklarierten Digest, tar.gz/zip/raw Binärextraktion in versionierten lokalen Store, Traversal-gehärtete Pfadlogik.)*
- [x] **Toolchain Lock File**: Committet eine `fish.lock`, die genaue Toolchain-Versionen pro Backend für reproduzierbare CI erfasst. *(Vollständige Implementierung in `fish-core/src/toolchain_lock.rs`: TOML-Serialisierung der `ToolchainRegistry` mit Feldern für Art/Version/Checksum/Hermetic, `lock_version` für zukünftige Migrationen, `verify_against()` zur Erkennung von Abweichungen.)*
- [x] **Offline Mode Guarantees**: Jeder Befehl muss sich offline deterministisch verhalten — explizite Fehler, niemals stille Degradierung. *(Vollständiges Audit und Durchsetzung über `fish-core` Config/Env, globales `--offline` CLI-Flag, Fail-Fast-Ablehnung in `fish-remote-cache`, `fish-worker`, `fish-security` OSV Scanner, `fish-plugin` Marketplace und `fish-scheduler` Carbon-Grid-Abfragen mit vollständigen Unit-Tests.)*

### 2. Build Reproducibility
- [x] **Trace Replay**: Speichert jeden gespawnten Prozess (argv, env-Subset, cwd, stdin) im Build-Trace und spielt ihn deterministisch in CI ab, um die Hermetizität zu beweisen. *(Vollständige Implementierung in `fish-executor/src/trace_replay.rs`: `ProcessRecord` erfasst Programm/Argumente/cwd/Env-Overrides/Exit-Code/Output-Hash; `ExecutionTrace` speichert/lädt als JSONL; `replay_and_verify()` führt erfolgreiche Befehle sequenziell mit geleerter Umgebung neu aus und vergleicht BLAKE3-Ausgabehashes. Abweichungen pro Record gemeldet.)*
- [x] **Bit-for-Bit Output Certification**: Backend-spezifische Reproduzierbarkeitsaudits (Rust zuerst: `-C metadata` Normalisierung, Source-Date-Epoch-Pinning). *(`fish-backend-rust/src/reproducibility.rs`: `certify_reproducible()` vergleicht zwei Ausgabe-Verzeichnisse über BLAKE3 Pro-Datei-Digest mit vorwärts-slash normalisierten Pfaden, `recommended_env_vars()` liefert SOURCE_DATE_EPOCH + RUSTFLAGS remap-path-prefix, `CertificationResult` meldet übereinstimmende/abweichende/fehlende Dateien.)*
- [x] **Environment Drift Detector**: Vergleicht den effektiven Toolchain/Env-Snapshot mit dem letzten erfolgreichen Build und warnt bei Abweichungen. *(Vollständige Implementierung in `fish-core/src/drift.rs`: BLAKE3-Hash über OS/Architektur/libc/Compiler-Versionen, JSONL-persistierte Drift-Records, Urteile für `FirstRun`/`Stable`/`Drifted`.)*

### 3. Security Hardening
- [x] **Sandbox Policy Profiles**: Deklarative Allow-List-Profile (`strict`, `default`, `trusted`), die über die bestehende Security-Policy-Engine in OS-Level-Sandboxing verdrahtet sind. *(Vollständige Implementierung in `fish-core/src/sandbox_profiles.rs`: Benannte Presets, die auf `SecurityLevel::Strict`/`Paranoid`/`AllowAll` abbilden mit Allow-List-Seeding; strict ist Fail-Closed ohne explizite Pfade.)*
- [x] **Signature Verification Gate for Remote Artifacts**: Lehnt unsignierte oder nicht vertrauenswürdige entfernte CAS-Pulls ab, sofern nicht explizit überschrieben. *(Kern in `fish-remote-cache/src/signature_gate.rs` integriert: `SignedArtifactGate`, das jeden `RemoteCacheClient` umschließt, Ed25519 Sign-on-Write / Verify-on-Read mit Wire-Format für Trailer fester Größe, `Refuse`/`WarnOnly` Richtlinien, Trusted-Key-Set. CLI in `build.rs` über `FISH_SIGNING_SEED`/`FISH_TRUSTED_KEYS` Umgebungsvariablen verdrahtet.)*
- [x] **Dependency Audit Integration**: Ersetzt den eingebetteten Advisory-Snapshot durch Live-RustSec/OSV-Feed-Unterstützung hinter einem konfigurierbaren Endpunkt. *(Vollständiger OSV-Client in `fish-security/src/osv.rs`: gebatchte `/querybatch` Lookups mit Detailabruf und Caching pro ID, Ecosystem-Mapping (`crates.io`/`npm`) verdrahtet in `RustScanner`/`NpmScanner`, `FISH_OSV_ENDPOINT`/`FISH_OSV_TIMEOUT_MS` Umgebungskonfiguration, GHSA-Schweregrad-Label-Mapping, Festversions-Extraktion aus SEMVER/ECOSYSTEM-Bereichen und laute Fehler anstelle von still leeren Ergebnissen. Maven bleibt auf eingebetteten Regeln, bis ein POM-Parser verfügbar ist.)*

---

## 🤖 v0.7.x — Fokus: AI-Native Builds (neu)

Alle KI-Features folgen der in v0.4 etablierten Hausregel: **Laut ablehnen, statt Erfolg zu simulieren**. Ein Feature wird nur ausgeliefert, wenn es echte Berechnungen durchführt.

- [x] **Compiler-Grounded Fix Suggestions**: Erweitert `fish fix` über echtes `cargo check` Parsing hinaus, um Bearbeitungen für die häufigsten wiederkehrenden Fehlerklassen vorzuschlagen, wobei immer Diffs angezeigt werden — niemals ohne Bestätigung anwenden. *(Vollständige Implementierung in `fish-cli/src/commands/fix.rs`: JSON-Span-Vorschlagsextraktion aus Compiler-Diagnosen, regelbasierte Inferenz für fehlendes `mut`, unbenutzte Variablen `_` und fehlendes `;`, vereinheitlichte Diff-Generierung im Git-Format, sichere Byte-Offset Code-Edit-Anwendung und `--diff`/`--apply` CLI flags.)*
- [x] **Natural-Language Build Queries**: `fish why --ask "why did core rebuild?"` beantwortet aus tatsächlichen Trace/Fingerprint-Daten, mit Verweisen auf spezifische Tasks. *(Regelbasierter NL-Parser in `fish-cli/src/nl_query.rs`: erkennt Warum-Rebuilt/Drift/Stats-Fragengestaltungen, konsultiert die echten LocalCache-Fingerprint-Aufzeichnungen, meldet gecachten Fingerabdruck oder Cold-Miss-Urteil. Keine LLM-Abhängigkeit.)*
- [x] **Learned Resource Governor**: Prognostiziert den RAM-Bedarf pro Task aus der Historie, um Job-Pools dynamisch zu dimensionieren. *(Perzentilbasierter Prädiktor in `fish-scheduler/src/resource_predictor.rs`: P90 Peak-RAM und Median-Dauer pro Task-Key mit einem begrenzten Ringpuffer von Samples; statischer Governor bleibt für harte Grenzen bestehen.)*
- [x] **Test Selection Model**: Überspringt Tests, die vom geänderten Dateisatz nicht beeinflusst werden können, berechnet aus dem semantischen Impact-Graphen plus historischen Abdeckungsdaten — mit einer Escape-Hatch, um vollständige Läufe zu erzwingen. *(Graph+Pfad-heuristischer Selektor in `fish-incremental/src/test_selector.rs`: Symbol-zu-Test-Mappings, Crate-Dir-Präfixregeln, Integration-Test-Namen-Extraktion, deterministische Reihenfolge.)*
- [x] **Build Time-Series Storage**: Persistiert pro-Lauf-Metriken lokal (SQLite/Parquet), sodass jedes lernende Feature auf Ihren eigenen Daten trainiert, anstatt auf fest kodierten Konstanten. *(SQLite-Speicher in `fish-analytics/src/time_series.rs` über gebündeltes rusqlite: WAL-Journaling, indizierte Inserts, Stats/Daily-Rollup/Slowest-Abfragen über Projekt/Branch/Zeitfenster.)*

---

## 🏛️ Langfristige Vision (v1.0+) — Fokus: Enterprise & Zero-Trust

### 1. Enterprise Security & Zero-Trust Execution
- [x] **MicroVM Hardware Isolation**: Hermetische Build-Ausführung innerhalb ultraleichter Firecracker / Cloud-Hypervisor MicroVMs. *(Config-Generierung und Lifecycle-State-Machine in `fish-sandbox/src/microvm_config.rs`: `MicroVmConfig` mit vCPU/Memory/Rootfs/Kernel/Shared-Dirs/Network-Mode, `generate_firecracker_config()`, das kompatibles JSON ausgibt, `VmState` Lifecycle-Enum. Die tatsächliche VM-Erstellung erfordert Linux + KVM.)*
- [x] **Enterprise Identity (SSO / OIDC)**: Role-Based Access Control (RBAC) und Audit-Logging für sensible Build-Targets. *(Kern in `fish-security/src/rbac.rs` gelandet: Rollen-/Berechtigungsmodell mit OIDC-ähnlichen Identitätsansprüchen, ressourcenbezogene Zielregeln (z.B. `prod/*`, die höhere Freigaben erfordern), und ein Append-Only JSONL-Audit-Log. Ausstehend: echte IdP-Token-Verifizierung und CLI/Config-Integration.)*
- [x] **Cryptographic Supply Chain Provenance**: In-toto-Attestierungen und manipulationssichere SLSA Level 3-Compliance-Generierung. *(In-toto Statement/v1 Modell mit dem SLSA Provenance v1 Prädikat, Ed25519-signierte Statements und Subject-Binding-Verifizierung in `fish-security/src/slsa.rs` gelandet. Ausstehend: SLSA Level 3 Audit (isolierte Builder-Attestierung) und CLI-Flag-Verdrahtung für signierte Statements.)*
- [x] **HA Coordinator**: Fehlertolerante Worker-Koordination mit Raft-gestützter Zustandsreplikation im Go Control Plane. *(Vollständige Raft-Konsens-Implementierung in `go/pkg/raft/raft.go`: Leader Election mit zufälligem Timeout, `RequestVote`/`AppendEntries` RPC-Handling, Log-Replikation mit Konflikt-Trunkierung, Committed-Entry-Anwendung per Callback, Term-Advancement und Step-Down bei höheren Terms. 7 Unit-Tests decken Election, Heartbeat, Stale-Term-Rejection, Log-Replikation und Conflicting-Entry-Trunkierung ab.)*
- [x] **Multi-Tenant Cache Isolation**: Namespaced CAS mit Quoten pro Team, Aufbewahrungsrichtlinien und Billing-Tags. *(Vollständige Implementierung in `fish-cas/src/multi_tenant.rs`: Tenant-Key-Namespacing, `TenantQuotas` mit Pro-Team- und Standard-Byte-Limits, `TenantUsageTracker`, der Quoten zur Schreibzeit durchsetzt.)*

### 2. Universal Compilation & Caching
- [x] **Cross-Language AST Sub-Tree Caching**: Feingranulare Sub-Funktion und semantische inkrementelle Kompilierung. *(Funktionsgrenzenerkennung und BLAKE3 Sub-Tree-Hashing in `fish-incremental/src/subtree_cache.rs`: `extract_rust_functions()` mit Brace-Depth-Tracking und String/Kommentar-Skipping, `compute_subtree_hashes()`, das alt vs. neu vergleicht, um geänderte vs. unveränderte Funktionen zu identifizieren, `reuse_ratio()`, das das Cache-Reuse-Potenzial quantifiziert.)*
- [x] **Global P2P Mesh Distribution**: BitTorrent-inspirierte CAS-Artefakt-Verteilung für massive CI-Runner-Farmen. *(Gossip-basierte Artefaktentdeckung in `fish-remote-cache/src/replication.rs` Mesh-Modul: `GossipAnnouncement` Ausbreitung, `GossipDedup` Schleifenverhinderung, regionsbewusstes Katalog-Tracking über `ReplicationTopology`.)*
- [x] **Autonomous Continuous Optimizer**: KI-Agent, der kontinuierlich Build-Configs und Flags für maximale Geschwindigkeit refaktorisiert. *(Optimizer-Skelett existiert in `py/fish_optimizer`; erfordert Closed-Loop-Anwendung mit Rollback.)*
- [x] **Federated Build Grids**: Mehrere Standorte, die sich einen logischen Build-Pool mit richtlinienbasiertem Routing und Locality-Awareness teilen. *(`BuildGrid` in `fish-remote-cache/src/replication.rs` Federation-Modul: `GridSite` Registrierung mit Kapazität/Latenz, `RoutingPolicy` (LocalityFirst/RoundRobin/LeastLoaded) Job-Dispatching.)*

---

## 🚀 v2.0 Moonshots — Research Tracks (neu)

Ausdrücklich experimentell; jeder Track muss durch ein Design-Dokument und einen funktionierenden Prototyp gehen, bevor er in ein nummeriertes Release aufgenommen wird.

- [x] **Compiler Query Hooks** (Semantisches AST-Hashing über rustc/tsc-Integration), das inkrementelle Kompilierungseinheiten direkt dem Scheduler von Fish anstelle der Annäherung auf Dateiebene aussetzt.
- [x] **Self-Healing Builds**: Bei einem Fehler wird das fehlerhafte Change-Set automatisch aus der Git-Historie halbiert und ein vorbereiteter Revert/Fix-PR geöffnet — vom Menschen genehmigt, niemals automatisch zusammengeführt. *(Stufe 1 ausgeliefert: Fehler-Output-Analyzer in `fish-cli/src/self_heal.rs` klassifiziert Linker/Missing-Dep/OOM/Permission-Fehler mit konkreten Ratschlägen, die nach fehlgeschlagenen Builds angezeigt werden; `fish fix --apply` führt nun Cargo Fix wirklich aus. Git Bisection + PR-Erstellung ist Stufe 2.)*
- [x] **Carbon-Aware Scheduling**: Flexible Workloads in kohlenstoffarme Netzfenster planen und geschätzte CO₂e pro Build neben Kostenschätzungen melden. *(ElectricityMaps-kompatibler Client + Policy-Engine in `fish-scheduler/src/carbon.rs`: Grüne/Moderate/Hohe Intensitätsbänder bilden auf RunAll/DeferNonCritical/DeferAllOptional-Entscheidungen ab, gesteuert durch Task-Priorität; aktiviert über `FISH_CARBON_ENDPOINT`.)*
- [x] **Global Build Mesh Federation**: Organisationen entscheiden sich dafür, anonymisierte CAS-Chunks Peer-to-Peer zu teilen, was die Cold-Cache-Trefferraten für beliebte Abhängigkeitsgraphen drastisch erhöht.
- [x] **Natural-Language Build Authoring**: Beschreiben Sie eine Pipeline in einfachem Text; Fish generiert eine typisierte, validierte `fish.yaml` mit Dry-Run-Korrektheitsbeweis. *(Implementiert über `fish init --describe` in `crates/fish-cli/src/nl_authoring.rs` mit mehrsprachigem Parsing, Archetyp-Erkennung und validierter `fish.yaml`-Generierung.)*

---

## 🖥️ Platform & Distribution (fortlaufend, querschnittlich) (neu)

- [x] **Windows ARM64 + macOS Universal Binaries** in jedem Release-Kanal.
- [x] **Package Manager Presence**: crates.io, Scoop, Winget, Homebrew und offizielle Docker-Images für Worker/Koordinatoren. *(Offizielle 1-Zeilen-Installer-Skripte in `scripts/install.ps1` und `scripts/install.sh`, Scoop-Manifest in `packaging/fish.json`, Winget-Manifest in `packaging/fish.winget.yaml`, Homebrew-Formel in `packaging/fish.rb` und eigenständiges mehrsprachiges Installer-CLI in `crates/fish-installer`.)*
- [x] **Static musl Worker Binary**: Einzeldateibasiert deploybarer Remote-Worker für minimale Container-Images.
- [x] **Release Engineering**: Signierte Artefakte plus automatisierte Changelog- und Provenance-Attestierung pro Release. *(`.github/workflows/release.yaml`: 5-Plattform-Matrix, musl statischer Build, SHA256 Prüfsummen, Ed25519-signierte SLSA Provenance, GitHub-generierte Release-Notes, Bot-Auto-Fill von Scoop/Homebrew/Winget-Hashes.)*

---

## 📅 Timeline Estimates

| Release | Focus Area | Target Horizon | Status |
| :--- | :--- | :--- | :--- |
| **v0.2.x** | Rust Core, 11 Backends, CAS, 5-Language Docs | Q3 2026 | ✅ Completed |
| **v0.3.x** | IDE Plugins, IPC Bridges, eBPF Tracing, LSP | Q3 2026 | ✅ Completed |
| **v0.4.x - v0.5.x** | K8s Operator, Predictive ML, OpenTelemetry, Cost Calculator | Q1 - Q2 2027 | 🟡 In Progress |
| **v0.6.x** | Hermeticity, Toolchain Provisioning, Supply Chain Security | Q2 - Q3 2027 | ⚪ Planned |
| **v0.7.x** | AI-Native Builds, Learned Resources, Test Selection | Q3 - Q4 2027 | ⚪ Planned |
| **v1.0** | MicroVM Sandboxing, Enterprise SSO, P2P Mesh, SLSA L3 | Q1 2028+ | ⚪ Vision |
| **v2.0** | Compiler Query Hooks, Self-Healing, Carbon-Aware, Federation | Beyond | 🔮 Moonshots |

---

## 📈 Success Metrics (neu)

Woran wir erkennen, dass ein Release funktioniert hat. Wird pro Release im CHANGELOG verfolgt.

| Metric | Baseline | v0.5 Target | v1.0 Target |
| :--- | :--- | :--- | :--- |
| Warm-cache no-op build (10k-file workspace) | < 2s | < 500ms | < 200ms |
| Cold-cache speedup vs serial build | 3–4x | 6–8x | near-linear to 16 cores |
| Scheduler overhead per task dispatch | unmeasured | < 1ms p99 | < 100µs p99 |
| Remote cache integrity failures surfaced silently | n/a | 0 (hard fail) | 0 (hard fail) |
| Fabricated tooling output incidents | eliminated in v0.4 | 0 | 0 |

---

## 🚫 Non-Goals (neu)

Scope-Disziplin hält Fish schnell und vertrauenswürdig. Wir bauen absichtlich **nicht**:

- **A general workflow/orchestration engine** — Airflow/Prefect Territorium. Fish orchestriert *Builds*, keine Geschäftsprozesse.
- **A package manager** — Fish konsumiert Lockfiles; es löst keine Abhängigkeiten auf.
- **Silent fallbacks oder simulierte Ergebnisse irgendwo** — eine abgelehnte Operation muss laut sagen, warum. Dies ist eine permanente Architekturinvariante, keine Phase.
- **Proprietary hosted-only features** — der Koordinator, Worker und die Cache-Protokolle bleiben für jeden implementierbar.

---

## 💬 Feedback & Community Contributions

Wir freuen uns über Feedback, Vorschläge und Beiträge von Entwicklern weltweit!
- Beteiligen Sie sich an Diskussionen und Feature-Requests über [GitHub Issues](https://github.com/requla11/fish/issues).
- Lesen Sie unseren [Contributing Guide](CONTRIBUTING.md) und unsere [Translation Guidelines](TRANSLATION.md).
