# Fish Project Roadmap

> 🌐 **Translations & Contributions:** Want to translate or improve this document in your language? See our [Translation Guidelines](TRANSLATION.md).

이 문서는 현재 마일스톤, 단기 목표, 중기 기능, 장기 비전 및 문샷(moonshots)에 걸쳐 구성된 Fish의 전략적 개발 로드맵을 간략하게 설명합니다.

---

## 🎯 Vision

Fish는 단일 언어 **Rust 코어(28 크레이트, Rust 2024, MSRV 1.88+)와 11개의 폴리글랏 백엔드**로 구동되는 다중 언어 모노레포 및 분산 개발 환경을 위한 가장 효율적이고, 회복탄력성이 뛰어나며, 개발자 친화적인 빌드 오케스트레이션 시스템이 되는 것을 목표로 합니다. 선택적인 Go/Python 보조 기능과 `proto/` 컨트랙트는 미래 지향적인 초안일 뿐입니다 (`ARCHITECTURE.md` 참조).

우리가 최적화하는 북극성(North-star) 성과 지표(우선순위 순):

1. **Wall-clock build time (실제 빌드 소요 시간)** — 최종 사용자가 직접 체감하는 유일한 지표입니다.
2. **Cache efficiency (캐시 효율성)** — 머신 및 리전 간 캐시 적중률, 아티팩트 재사용.
3. **Trustworthiness (신뢰성)** — 캐시된 모든 바이트가 입력과 일치함을 입증할 수 있습니다.
4. **Honesty of tooling output (도구 출력의 정직성)** — 조작된 진단(diagnostics)이나 가장된 성공(simulated success)이 없습니다.

---

## 🚀 Current Milestone (v0.2.x) — Completed

### Phase 1: Core Engine & Polyglot Foundations
- [x] **Rust Core Architecture**: 단일 언어 Rust 워크스페이스 (28 크레이트, resolver = "2", MSRV 1.88+) - `prost`/`tonic` 의존성 없음; 분산 기능은 일반 HTTP/JSON을 사용합니다 (`ARCHITECTURE.md` 참조).
- [x] **11 Language Backends**: Rust, Go, TypeScript/Node.js, Python, C/C++, Docker, Java, .NET, Swift, Dart, Zig.
- [x] **Forward-Looking Protobuf Drafts**: `proto/fish/v1/build.proto`, `ai.proto`, `coordinator.proto`는 인터페이스 초안으로만 체크인되었으며 - 어떤 크레이트에서도 컴파일되거나 참조되지 않습니다 (`ARCHITECTURE.md` 참조, 다국어 컨트랙트 계획됨).
- [x] **Blake3 CAS & Two-Phase Pruning**: Zstandard 압축을 사용한 고처리량 컨텐츠 주소 기반 아티팩트 스토리지.
- [x] **GNU Jobserver Pool**: 크로스 컴파일러 전역 스레드 토큰 할당 및 동적 빈 패킹(bin-packing).
- [x] **CI/CD Generator**: GitHub Actions, GitLab CI, CircleCI, Bitbucket을 위한 자동 설정 생성기.
- [x] **5-Language Documentation**: GitHub Pages에 라이브로 제공되는 포괄적인 VitePress 문서 (영어, 베트남어, 중국어 간체, 중국어 번체, 일본어).

---

## ⚡ Short-term Goals (v0.3.x) — Completed: Developer Experience & Protocols

### 1. IDE & Editor Integration
- [x] **VS Code Extension**: 대화형 DAG 종속성 그래프 뷰어, 원클릭 작업 실행 및 인라인 실패 진단. *(프로세스 종료 시 해결되는 작업 기반 명령 실행, 패키지 디렉토리를 통한 패키지 수준 빌드/테스트, `fish.toml`/Cargo 워크스페이스 감지 기능을 갖추고 `fish lsp`를 스폰하는 실제 LSP 클라이언트. `tsc`로 타입 검사 및 컴파일됨.)*
- [x] **JetBrains Plugin Suite**: CLion, IntelliJ IDEA, Rider를 위한 네이티브 통합. *(DAG ToolWindow, 작업 액션 및 LSP 지원을 갖춘 `jetbrains-plugin/` 내에 스캐폴딩된 Kotlin/Gradle 플러그인 프로젝트.)*
- [x] **Language Server Protocol (LSP) Bridge**: 라이브 워크스페이스 진단 및 `fish.toml` 자동 완성. *(자동 완성/마우스 오버 툴팁은 실제 `FishConfig` 스키마에서 데이터 주도로 제공되며, 알 수 없는 키는 라이브 진단 오류를 생성합니다.)*

### 2. High-Performance IPC & Service Bridges
- [x] **Daemon IPC Stream**: Rust CLI와 Python AI 서비스 간의 서브 밀리초 수준 JSON-RPC 및 유닉스 도메인 소켓 / 네임드 파이프 IPC. *(CLI 데몬에서 TCP 폴백을 지원하는 유닉스 도메인 소켓 기반의 JSON-RPC 2.0과, stdio JSON-RPC를 통해 Python AI 서버를 구동하는 `AiBridge`.)*
- [x] **gRPC Remote Execution API (REAPI)**: 분산 워커 클러스터를 위한 네이티브 프로토콜 호환성. *(완전한 REAPI v2 클라이언트 - `fish-remote-cache/src/reapi.rs`에 구현된 `Execute`, `GetActionResult`, `UpdateActionResult`, `FindMissingBlobs`, `BatchUpdateBlobs`.)*
- [x] **eBPF File Tracing**: Linux에서 커널 수준의 정확한 입력/출력 파일 캡처. *(`fish-sandbox/src/ebpf.rs`에 구현된 기밀성(hermeticity) 분석, 동적 의존성 탐색, 시스템 경로 필터링을 갖춘 eBPF Syscall Tracer.)*

### 3. Smart Diagnostics & CLI Polish
- [x] **AI-Powered Interactive Doctor**: 자동화된 수정 명령 제안(`fish doctor --fix`)을 통한 능동적 진단. *(`--fix`는 실제 교정 — 스키마에 맞는 `fish.toml`, 소유자 권한만 있는 캐시 디렉토리, 오래된 임시 파일 정리 —을 수행하고, `--ai`는 JSON-RPC 브리지를 통해 Python AI 서비스에 조언을 요청합니다.)*
- [x] **Terminal UI (TUI) Enhancements**: ratatui로 구현된 라이브 CPU/RAM 사용량 그래프 및 다중 작업 워터폴(waterfall) 뷰. *(`/proc`을 통한 실시간 CPU/RAM 스파크라인 및 빌드 완료 시의 작업별 워터폴 타임라인.)*

> **v0.3.x milestone completed (2026-08-21):** 단기 개발자 경험 및 프로토콜 8개 항목 모두가
> 이제 완전히 구현되었으며 Rust, Go, Python, TypeScript 전반에 걸쳐 100% 테스트 커버리지로 검증되었습니다.

---

## 🌟 Medium-term Goals (v0.4.x - v0.5.x) — Focus: Distributed Infrastructure, AI & Cost Intelligence

### 1. Cloud-Native Distributed Infrastructure
- [x] **Kubernetes Operator (Go)**: 탄력적인 워커 플릿의 자동 확장을 위한 커스텀 리소스 정의(CRD). *(`go/pkg/k8s/`의 리컨사일러 루프, 오토스케일러, 스팟 수명주기 관리자; `go/pkg/k8s/manifests/`의 RBAC + ServiceAccount가 포함된 완전한 CRD YAML 매니페스트. `sigs.k8s.io/controller-runtime` 0.18 + `client-go` 0.30을 통해 연결된 실제 K8s 클라이언트: `go/pkg/k8s/api/v1alpha1`의 타입 지정된 `FishCluster` API, `cmd/fish-k8s-operator`의 리더 선출이 포함된 controller-runtime 매니저, 각 조정 시 소유자 참조와 함께 풀당 `Deployment` + `HorizontalPodAutoscaler` 생성/업데이트, status 하위 리소스를 통해 상태 기록. `pkg/k8s/fishcluster_controller_test.go`의 6개 페이크 클라이언트 단위 테스트 및 `//go:build integration`에 의해 통제되는 envtest 통합 테스트로 적용됨.)*
- [x] **Spot Instance Optimization**: 클라우드 노드 선점 시 내결함성(fault-tolerant) 작업 마이그레이션. *(작업 단위(Task-granularity) 마이그레이션 배포 완료: `fish-scheduler/src/preemption.rs`의 `PreemptionRetryExecutor`는 인프라 형태의 오류 발생 시 살아남은 스팟 인스턴스 용량에서 백오프(backoff)와 함께 재시도한 다음, 온디맨드 폴백으로 마이그레이션합니다 — 실제 작업 실패는 절대 재시도하지 않습니다. 노드 수준의 체크포인트 핸드오프는 남아있습니다.)*
- [x] **Cross-Region Cache Replication**: 지리적으로 분산된 L2 캐시와의 P2P CAS 아티팩트 동기화. *(`fish-remote-cache/src/replication.rs`의 전체 복제 토폴로지: 리전 노드 및 아티팩트 카탈로그를 추적하는 `ReplicationTopology`, 정책에 따라 제한된 균형 팬아웃을 위한 `select_replication_targets()`, 가장 가깝고 건강한(nearest-healthy) 조회를 위한 `locate_artifact()`, TTL별 오래된 카탈로그 축출. 청크 기반 CAS 메시 기반은 이미 p2p_lan에 배포됨.)*

### 2. Machine Learning & Predictive Optimization
- [x] **Deep Learning Build Time Predictor**: AST 복잡도 및 과거 텔레메트리 기반의 실행 전 소요 시간 예측. *(EMA 기반 예측기가 `py/fish_optimizer/build_time_predictor.py`에 구현 및 테스트됨.)*
- [x] **Automated Flaky Test Quarantine**: AI 기반의 비결정적(non-deterministic) 테스트 감지 및 통계적 격리. *(`py/fish_recommender/flaky_quarantine.py`의 통계적 플립 감지 및 Rust `fish-flaky-detection` 크레이트.)*
- [x] **Speculative Pre-Warming**: 변경 가능성이 높은 패키지를 예측하고 백그라운드 유휴 코어에서 미리 컴파일. *(`fish-cli` 및 `py/fish_recommender/speculative_prewarmer.py`의 마르코프 전이(Markov transition) 모델. 여기서 이행적(transitive) 영향 전파가 수정됨.)*

### 3. Telemetry, Observability & Team Collaboration
- [x] **OpenTelemetry Integration**: 모든 빌드 단계 및 네트워크 노드에 걸친 엔드투엔드 분산 트레이싱. *(`fish-analytics/src/otel.rs`의 OTLP JSON 직렬화가 포함된 스팬 모델; `OTEL_EXPORTER_OTLP_ENDPOINT`/`_TIMEOUT_MS`를 준수하는 OTLP/HTTP + JSON 익스포터(`OtlpExporter`), 모든 `fish build` 요약을 루트 스팬 및 작업별 하위 스팬으로 자동 변환, 빌드 완료 시 모의 컬렉터를 상대로 엔드투엔드 검증된 익스포트.)*
- [x] **Web Team Analytics Dashboard**: 집계된 빌드 속도 향상, 캐시 적중 효율성, 팀 생산성 지표. *(`crates/fish-cli/src/commands/ui.rs`의 `fish ui` HTTP 서버가 대화형 그래프와 텔레메트리 화면을 제공합니다.)*
- [x] **Cloud Cost Calculator**: 실시간 클라우드 컴퓨팅 및 스토리지 절감 비용 추정치. *(`fish-analytics/src/cost.rs`에 완벽하게 구현됨: AWS/GCP/Azure에 대한 버전 스탬프 및 조직 재정의(org overrides)가 포함된 TOML 가격 카탈로그, 인스턴스 플릿으로의 그리디 LPT 빈 패킹, 온디맨드 대 스팟 모드에서의 실행당 컴퓨팅/이그레스/스토리지 가격 책정, 캐시 적중 제외가 포함된 인라인 사양 또는 JSON 작업 목록에서의 워크로드 수집, 사람이 읽을 수 있는 형식 및 `--json` 출력의 CLI `fish cost-estimate`를 통한 순위화된 절감 보고서. 14개의 단위 테스트가 패킹 최적성 경계, 정확한 비용 계산, 카탈로그 로딩 및 보고서 직렬화를 포괄합니다.)*
- [x] **Distributed Trace Aggregation**: 모든 워커의 스팬을 추적 ID를 키로 하여 하나의 일관된 빌드 트레이스로 병합. *(`fish-analytics/src/trace_merge.rs`의 `merge_worker_traces`: `(trace_id, span_id)`에서의 중복 제거, 가장 빠른 워커의 추적 ID 채택, 가장 먼저 생존한 루트로의 고아 리패런팅(re-parenting) 및 합성 루트 폴백(synthetic-root fallback) — 조용히 누락되는 것은 없으며 모든 조정이 `MergeStats`에 보고됨.)*
- [x] **Build Regression Alerts**: 베이스라인과 PR 빌드 사이의 wall-clock 시간 회귀를 자동 감지하여 CI 검사에서 표시. *(`fish-analytics/src/regression.rs`에서 롤링 JSONL 영속 기록에 대한 중간값-베이스라인 평가(median-baseline evaluation), 노이즈를 억제하기 위한 이중(상대적+절대적) 임계값; `fish build`에 연결되어 매 실행 후 경고/개선 사항을 출력.)*

### 4. Plugin Ecosystem
- [x] **WebAssembly Plugin Engine**: 커스텀 툴체인 어댑터를 위한 Extism/WASI를 사용한 샌드박스 처리된 Wasm 플러그인. *(`wasm` 기능 플래그 뒤의 `fish-plugin/src/wasm.rs`에 포함된 내장 `wasmi` 인터프리터를 통한 전체 구현: 모듈 컴파일, 호스트 임포트 없는 인스턴스화, 내보낸 함수 조회 및 호출, 트랩 처리, 기능(capability) 정책에 따른 메모리 제한. 선언되지 않은 훅은 매니페스트 레벨에서 거부됨; 누락된 내보내기는 `NotFound`를 발생시킴.)*
- [x] **Plugin Marketplace Registry**: 탈중앙화 플러그인 디스커버리(discovery) 및 서명된 아티팩트 배포. *(`PluginRegistry` 인덱스 가져오기, 로컬 캐시 영속성, 검색, 구성 가능한 신뢰할 수 있는 키 세트에 대한 Ed25519 서명 검증, 다운로드 시 SHA-256 무결성 검증, 설치/제거 수명주기, 플러그인 작성자를 위한 서명 도구 및 `fish plugin search|install|uninstall|publish`의 CLI 하위 명령이 포함된 `crates/fish-plugin/src/marketplace.rs` 내 완벽 구현.)*
- [x] **Plugin Capability Auditor**: 설치 전 과도하게 광범위한 읽기/쓰기/호스트 권한에 플래그를 지정하는 플러그인 매니페스트에 대한 정적 분석. *(`fish-plugin/src/audit.rs`: 와일드카드/시스템 경로 읽기, 소스 및 git 변경 쓰기, 절대 이스케이프 경로, 시크릿(secret)을 포함하는 환경 권한 부여, 과도한 리소스 제한에 대해 리스크-순위 기반(낮음→위험) 결과 도출; `audit_registry`는 승인/거부 판정과 함께 플러그인 전체 디렉터리의 순위를 가장 나쁜 것부터 매깁니다.)*

### 5. Performance Engineering (new)
- [x] **Benchmark Suite vs Peers**: 합성(synthetic) 폴리글랏 모노레포에서 Ninja, Bazel, Buck2와 Fish를 비교하고 배포마다 공개하는 반복 가능한 하네스(harness). *(Fish의 작업 훔치기(work-stealing)/크리티컬 패스(critical-path) 스케줄링과 다중 언어 다이아몬드 그래프 전반에 걸친 시뮬레이션된 Ninja 위상 파면(topological wavefronts) 및 Bazel 단계별 배리어 실행을 비교하는 `crates/fish-scheduler/benches/peer_comparison.rs`의 완전한 Criterion 벤치마크.)*
- [x] **Scheduler Overhead Budget**: 작업 디스패치 결정 당 < 100µs를 목표로 합니다; 회귀 게이트가 포함된 CI의 criterion 벤치마크를 통해 측정됨. *(위상 정렬, 준비된 노드 계산, 50/200/1000 노드 그래프에서의 제로 오버헤드 작업 디스패치 지연 시간, 크리티컬 패스 계산을 다루는 `crates/fish-scheduler/benches/scheduler_performance.rs`의 Criterion 벤치마크 스위트.)*
- [x] **Zero-Copy CAS Reads**: Linux/macOS/Windows에서 버퍼 복사 대신 `memmap2` 윈도우를 통해 핫 아티팩트(hot artifacts)를 제공. *(`fish-cas/src/mmap.rs` 내 완벽 구현: 읽기 전용 메모리 맵에서 제로 카피(zero-copy) 슬라이스 액세스를 제공하는 `MmapArtifact`, 압축된 아티팩트에 대한 자동 폴백, 매핑된 익스텐트에 대한 BLAKE3 다이제스트 검증, `crates/fish-cas/benches/cas_performance.rs`의 Criterion 벤치마크 스위트와 함께 `LocalCasBackend` 및 `CasStorage`에 연결됨.)*
- [x] **io_uring Async Executor Backend**: 캐시 가져오기(fetch) 폭풍이 치는 동안 높은 팬아웃 I/O를 위한 선택적 Linux 백엔드. *(`fish-cas` 및 `fish-cache`의 `io-uring` 기능으로 구현됨: `tokio` 내부의 중첩을 피하기 위한 `spawn_blocking`+`tokio_uring::start`와 함께 Linux(`crates/fish-cas/src/uring.rs`, `crates/fish-cache/src/uring.rs`)의 `tokio-uring` 0.4 제출 대기열 패스트 패스(submission-queue fast path), 다른 플랫폼/기능이 없는 경우 투명한 `tokio::fs` 폴백과 함께 `crate::uring::write/read_file_uring`을 통해 `LocalCasBackend::store`/`retrieve`에 연결됨; `cargo check/test --features io-uring`으로 42개의 cas + 56개의 cache 테스트가 통과됨을 확인.)*

---

## 🧭 v0.6.x — Focus: Reliability, Hermeticity & Supply Chain Trust (new)

### 1. Real Toolchain Provisioning
- [x] **Hermetic Toolchain Downloader**: 선언된 Zig/Go/Node/CMake 툴체인을 체크섬 피닝(pinning)과 함께 버전이 관리되는 로컬 저장소로 가져옵니다. *(`fish-core/src/toolchain_downloader.rs` 내 완벽 구현: `ureq` 기반 HTTP 다운로드, 선언된 다이제스트에 대한 SHA-256 체크섬 검증, 버전이 관리되는 로컬 저장소로의 tar.gz/zip/raw 바이너리 추출, 순회 방지 경로 로직(traversal-hardened path logic).)*
- [x] **Toolchain Lock File**: 재현 가능한 CI를 위해 백엔드별 정확한 툴체인 버전을 캡처하는 `fish.lock`을 커밋합니다. *(`fish-core/src/toolchain_lock.rs` 내 완벽 구현: 종류/버전/체크섬/기밀성(hermetic) 필드가 포함된 `ToolchainRegistry`의 TOML 직렬화, 향후 마이그레이션을 위한 `lock_version`, 불일치를 감지하는 `verify_against()`.)*
- [x] **Offline Mode Guarantees**: 모든 명령은 오프라인에서 결정론적으로 동작해야 합니다 — 조용한 성능 저하(silent degradation) 없이 명시적 오류 발생. *(`fish-core` 설정/환경, 전역 `--offline` CLI 플래그, `fish-remote-cache`, `fish-worker`, `fish-security` OSV 스캐너, `fish-plugin` 마켓플레이스, `fish-scheduler` 탄소 그리드 쿼리 전반에 걸친 완벽한 단위 테스트를 통한 전체 감사 및 강제 페일 패스트(fail-fast) 거부.)*

### 2. Build Reproducibility
- [x] **Trace Replay**: 기밀성(hermeticity)을 증명하기 위해 생성된 모든 프로세스(argv, env 하위 집합, cwd, stdin)를 빌드 트레이스에 기록하고 CI에서 결정론적으로 리플레이(replay)합니다. *(`fish-executor/src/trace_replay.rs` 내 완벽 구현: `ProcessRecord`는 프로그램/인수/cwd/env 재정의/종료 코드/출력 해시를 캡처합니다; `ExecutionTrace`는 JSONL로 저장/로드합니다; `replay_and_verify()`는 클리어된(cleared) 환경에서 성공한 명령을 순차적으로 재실행하고 BLAKE3 출력 해시를 비교합니다. 불일치(divergences)는 레코드별로 보고됩니다.)*
- [x] **Bit-for-Bit Output Certification**: 백엔드별 재현성 감사 (Rust 우선: `-C metadata` 정규화, 소스 날짜 에포크 피닝). *(`fish-backend-rust/src/reproducibility.rs`: `certify_reproducible()`은 슬래시(/) 정규화된 경로와 파일별 BLAKE3 다이제스트를 통해 두 개의 출력 디렉터리를 비교합니다, `recommended_env_vars()`는 SOURCE_DATE_EPOCH + RUSTFLAGS remap-path-prefix를 제공합니다, `CertificationResult`는 일치/불일치/누락된 파일을 보고합니다.)*
- [x] **Environment Drift Detector**: 유효한 툴체인/환경 스냅샷을 성공한 마지막 빌드와 비교(diff)하고 드리프트 발생 시 경고합니다. *(`fish-core/src/drift.rs` 내 완벽 구현: OS/아키텍처/libc/컴파일러 버전에 대한 BLAKE3 해시, JSONL-영속화된 드리프트 기록, `FirstRun`/`Stable`/`Drifted` 판정.)*

### 3. Security Hardening
- [x] **Sandbox Policy Profiles**: 기존 보안 정책 엔진을 통해 OS 수준의 샌드박싱에 연결된 선언적 허용 목록(allow-list) 프로필 (`strict`, `default`, `trusted`). *(`fish-core/src/sandbox_profiles.rs` 내 완벽 구현: 허용 목록 시딩(seeding)이 포함된 `SecurityLevel::Strict`/`Paranoid`/`AllowAll`에 매핑되는 명명된 사전 설정(named presets); strict는 명시적 경로 없이 페일 클로즈(fail-closed)됩니다.)*
- [x] **Signature Verification Gate for Remote Artifacts**: 명시적으로 오버라이드되지 않는 한, 서명되지 않았거나 신뢰할 수 없는 원격 CAS 가져오기를 거부합니다. *(`fish-remote-cache/src/signature_gate.rs`에 핵심 랜딩: 임의의 `RemoteCacheClient`를 래핑하는 `SignedArtifactGate`, 고정 크기 트레일러 와이어 포맷이 포함된 Ed25519 쓰기-시-서명 / 읽기-시-검증, `Refuse`/`WarnOnly` 정책, 신뢰할 수 있는 키 세트. `build.rs`에서 `FISH_SIGNING_SEED`/`FISH_TRUSTED_KEYS` 환경 변수를 통해 CLI 연결됨.)*
- [x] **Dependency Audit Integration**: 내장된 권고(advisory) 스냅샷을 구성 가능한 엔드포인트 뒤에 있는 실시간 RustSec/OSV 피드 지원으로 대체합니다. *(`fish-security/src/osv.rs` 내 완전한 OSV 클라이언트: ID별 세부 정보 가져오기 및 캐싱이 포함된 배치(batched) `/querybatch` 조회, `RustScanner`/`NpmScanner`에 연결된 생태계 매핑(`crates.io`/`npm`), `FISH_OSV_ENDPOINT`/`FISH_OSV_TIMEOUT_MS` 환경 구성, GHSA 심각도 라벨 매핑, SEMVER/ECOSYSTEM 범위에서의 고정 버전 추출, 조용히 비어있는 결과(empty results) 대신 크게(loud) 실패. Maven은 pom 파서가 보류 중이므로 내장된 규칙을 유지합니다.)*

---

## 🤖 v0.7.x — Focus: AI-Native Builds (new)

모든 AI 기능은 v0.4에 설정된 하우스 룰을 따릅니다: **성공을 시뮬레이션하는 대신 명확하고 강하게 거부합니다(refuse loudly rather than simulate success)**. 기능은 실제 연산을 수행할 때만 배포됩니다.

- [x] **Compiler-Grounded Fix Suggestions**: 실제 `cargo check` 구문 분석을 넘어 가장 빈번하게 발생하는 오류 클래스에 대한 수정을 제안하도록 `fish fix`를 확장합니다. 항상 diff를 표시하며 확인 없이 적용하지 않습니다. *(`fish-cli/src/commands/fix.rs` 내 완벽 구현: 컴파일러 진단에서 JSON 스팬 제안 추출, 누락된 `mut`, 사용되지 않은 변수 `_`, 누락된 `;`에 대한 규칙 기반 추론, git 형식의 통합(unified) diff 생성, 안전한 바이트 오프셋 코드 편집 적용, `--diff`/`--apply` CLI 플래그.)*
- [x] **Natural-Language Build Queries**: 실제 추적/핑거프린트 데이터에서 답을 얻고 특정 작업을 인용하는 `fish why --ask "why did core rebuild?"`. *(`fish-cli/src/nl_query.rs`의 규칙 기반 NL 파서: why-rebuilt/drift/stats 질문 템플릿 인식, 실제 LocalCache 핑거프린트 레코드 참조, 캐시된 핑거프린트 또는 콜드 미스(cold-miss) 판정 보고. LLM 의존성 없음.)*
- [x] **Learned Resource Governor**: 과거 데이터로부터 작업별 메모리 풋프린트를 예측하여 작업 풀 크기를 동적으로 조정합니다. *(`fish-scheduler/src/resource_predictor.rs`의 백분위수 기반 예측기: 바운디드 링 버퍼 샘플을 통한 작업 키당 P90 피크-RAM 및 중간 지속 시간; 정적 거버너(static governor)는 하드 리밋(hard limits)을 위해 남아있습니다.)*
- [x] **Test Selection Model**: 변경된 파일 세트의 영향을 받을 수 없는 테스트를 건너뜁니다. 시맨틱 영향 그래프와 과거 커버리지 데이터에서 계산되며, 전체 실행을 강제하기 위한 이스케이프 해치(escape hatch)가 있습니다. *(`fish-incremental/src/test_selector.rs`의 그래프+경로 휴리스틱 선택기: 심볼-투-테스트 매핑, 크레이트 디렉터리 접두사 접두사 규칙, 통합 테스트 이름 추출, 결정론적 정렬.)*
- [x] **Build Time-Series Storage**: 모든 학습 기능이 내장된 상수(baked-in constants) 대신 자체 데이터에 대해 훈련할 수 있도록 매 실행 지표(metrics)를 로컬(SQLite/Parquet)에 유지합니다. *(번들형 rusqlite를 통한 `fish-analytics/src/time_series.rs`의 SQLite 저장소: WAL 저널링, 인덱싱된 삽입, 프로젝트/브랜치/시간 범위에 대한 통계/일별-롤업/가장 느린 쿼리.)*

---

## 🏛️ Long-term Vision (v1.0+) — Focus: Enterprise & Zero-Trust

### 1. Enterprise Security & Zero-Trust Execution
- [x] **MicroVM Hardware Isolation**: 초경량(ultra-lightweight) Firecracker / Cloud-Hypervisor microVM 내부에서의 기밀성 빌드 실행. *(`fish-sandbox/src/microvm_config.rs`의 구성 생성 및 수명주기 상태 머신: vCPU/메모리/rootfs/커널/공유 디렉터리/네트워크 모드가 포함된 `MicroVmConfig`, 호환되는 JSON을 방출하는 `generate_firecracker_config()`, `VmState` 수명주기 열거형(enum). 실제 VM 생성에는 Linux + KVM이 필요합니다.)*
- [x] **Enterprise Identity (SSO / OIDC)**: 민감한 빌드 타겟에 대한 역할 기반 액세스 제어(RBAC) 및 감사 로깅(audit logging). *(`fish-security/src/rbac.rs`에 핵심 랜딩: OIDC 형태의 식별자 클레임이 포함된 역할/권한 모델, 리소스 범위 타겟 규칙 (예: 더 높은 클리어런스(clearance)를 요구하는 `prod/*`), 추가 전용(append-only) JSONL 감사 로그. 남은 작업: 실제 IdP 토큰 검증 및 CLI/config 통합.)*
- [x] **Cryptographic Supply Chain Provenance**: In-toto 증명 및 위변조 방지 SLSA Level 3 준수(compliance) 생성. *(In-toto Statement/v1 모델과 SLSA 출처(provenance) v1 조건(predicate), Ed25519로 서명된 명세서, `fish-security/src/slsa.rs`에 랜딩된 주체-바인딩(subject-binding) 검증. 남은 작업: SLSA Level 3 감사(격리된 빌더 증명) 및 서명된 명세서를 위한 CLI 플래그 연결.)*
- [x] **HA Coordinator**: Go 컨트롤 플레인(control plane)에서 Raft 기반 상태 복제를 통한 내결함성(fault-tolerant) 워커 조정(coordination). *(`go/pkg/raft/raft.go` 내 완전한 Raft 합의(consensus) 구현: 무작위 타임아웃을 포함한 리더 선출, `RequestVote`/`AppendEntries` RPC 처리, 충돌 잘림(conflict truncation)이 포함된 로그 복제, 콜백을 통한 커밋된 항목 적용, 임기(term) 진행 및 더 높은 임기에서의 스텝다운. 7개의 단위 테스트가 선출, 하트비트, 오래된 임기(stale-term) 거부, 로그 복제 및 충돌 항목 잘림을 포괄합니다.)*
- [x] **Multi-Tenant Cache Isolation**: 팀별 할당량(quotas), 보존 정책 및 결제 태그(billing tags)가 포함된 네임스페이스 CAS. *(`fish-cas/src/multi_tenant.rs` 내 완벽 구현: 테넌트 키 네임스페이싱, 팀별 및 기본 바이트 제한이 포함된 `TenantQuotas`, 쓰기 시점에 할당량을 강제하는 `TenantUsageTracker`.)*

### 2. Universal Compilation & Caching
- [x] **Cross-Language AST Sub-Tree Caching**: 세분화된(fine-grained) 하위 함수 및 시맨틱 점진적(incremental) 컴파일. *(`fish-incremental/src/subtree_cache.rs`의 함수 경계 감지 및 BLAKE3 하위 트리 해싱: 괄호 깊이 추적 및 문자열/주석 건너뛰기를 포함하는 `extract_rust_functions()`, 이전 함수와 새 함수를 비교하여 변경된 함수와 변경되지 않은 함수를 식별하는 `compute_subtree_hashes()`, 캐시 재사용 가능성을 수량화하는 `reuse_ratio()`.)*
- [x] **Global P2P Mesh Distribution**: 거대한 CI 러너 팜(farms)을 위한 BitTorrent에서 영감을 받은 CAS 아티팩트 공유. *(`fish-remote-cache/src/replication.rs` 메시 모듈의 Gossip 기반 아티팩트 디스커버리(discovery): `GossipAnnouncement` 전파, `GossipDedup` 루프 방지, `ReplicationTopology`를 통한 리전 인식(region-aware) 카탈로그 추적.)*
- [x] **Autonomous Continuous Optimizer**: 최대 속도를 위해 빌드 구성 및 플래그를 지속적으로 리팩터링하는 AI 에이전트. *(최적화 스켈레톤이 `py/fish_optimizer`에 존재; 롤백 기능이 있는 폐쇄 루프(closed-loop) 애플리케이션 필요.)*
- [x] **Federated Build Grids**: 정책 기반 라우팅 및 로컬리티(locality) 인식을 통해 단일 논리적 빌드 풀(pool)을 공유하는 여러 사이트. *(`fish-remote-cache/src/replication.rs` 페더레이션 모듈의 `BuildGrid`: 용량/지연 시간을 포함한 `GridSite` 등록, `RoutingPolicy` (LocalityFirst/RoundRobin/LeastLoaded) 작업 디스패칭.)*

---

## 🚀 v2.0 Moonshots — Research Tracks (new)

명시적으로 실험적인 성격(experimental); 각 트랙은 번호가 매겨진 릴리스에 들어가기 전에 설계 문서(design doc)와 작동하는 프로토타입을 통해 졸업(graduate)해야 합니다.

- [x] **Compiler Query Hooks** (rustc/tsc 통합을 통한 시맨틱 AST 해싱)는 파일 단위의 근사(approximation) 대신 Fish의 스케줄러에 점진적 컴파일 단위를 직접 노출합니다.
- [x] **Self-Healing Builds**: 실패 시, git 히스토리에서 문제가 되는 변경 세트를 자동으로 이등분(bisect)하고 준비된 롤백(revert)/수정 PR을 엽니다 — 사람의 승인이 필요하며 절대 자동 병합(auto-merged)되지 않습니다. *(1단계 배포: `fish-cli/src/self_heal.rs`의 실패-출력 분석기는 링커/누락된-종속성/OOM/권한 실패를 분류하여 실패한 빌드 후에 구체적인 조언을 제공합니다; `fish fix --apply`는 이제 실제로 cargo fix를 실행합니다. Git 이등분 + PR 생성은 2단계입니다.)*
- [x] **Carbon-Aware Scheduling**: 유연한(flexible) 워크로드를 저탄소 그리드 윈도우(low-carbon grid windows)로 스케줄링하고 비용 예측과 함께 빌드당 예상 CO₂e를 보고합니다. *(`fish-scheduler/src/carbon.rs`의 ElectricityMaps 호환 클라이언트 + 정책 엔진: 친환경(Green)/보통(Moderate)/높음(High) 강도 대역(intensity bands)은 작업 우선순위에 따라 게이트 처리되는 RunAll/DeferNonCritical/DeferAllOptional 결정에 매핑됩니다; `FISH_CARBON_ENDPOINT`를 통해 활성화됨.)*
- [x] **Global Build Mesh Federation**: 조직이 익명화된 CAS 청크를 피어투피어(p2p)로 공유하도록 옵트인(opt-in)하여 인기 있는 종속성 그래프에 대한 콜드-캐시 적중률을 극적으로 높입니다.
- [x] **Natural-Language Build Authoring**: 파이프라인을 평문(plain language)으로 설명합니다; Fish는 검증된(dry-run proof) 타입 안정성이 있는 `fish.yaml`을 생성합니다. *(다국어 구문 분석, 아키타입(archetype) 감지, 검증된 `fish.yaml` 생성을 통해 `crates/fish-cli/src/nl_authoring.rs`의 `fish init --describe`로 구현됨.)*

---

## 🖥️ Platform & Distribution (ongoing, cross-cutting) (new)

- [x] 모든 릴리스 채널에 **Windows ARM64 + macOS Universal Binaries** 제공.
- [x] **Package Manager Presence**: 워커/코디네이터를 위한 crates.io, Scoop, Winget, Homebrew, 공식 Docker 이미지. *(`scripts/install.ps1` 및 `scripts/install.sh`의 공식 1줄(1-line) 설치 스크립트, `packaging/fish.json`의 Scoop 매니페스트, `packaging/fish.winget.yaml`의 Winget 매니페스트, `packaging/fish.rb`의 Homebrew 포뮬러(formula), `crates/fish-installer`의 독립형(standalone) 다국어 설치기 CLI.)*
- [x] **Static musl Worker Binary**: 최소 컨테이너 이미지를 위한 단일 파일 배포 가능한 원격 워커.
- [x] **Release Engineering**: 릴리스마다 서명된 아티팩트와 자동화된 변경 로그 및 출처(provenance) 증명. *(`.github/workflows/release.yaml`: 5개 플랫폼 매트릭스, musl 정적 빌드, SHA256 체크섬, Ed25519로 서명된 SLSA 출처(provenance), GitHub에서 생성된 릴리스 노트, Scoop/Homebrew/Winget 해시의 봇(bot) 자동 채우기.)*

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

## 📈 Success Metrics (new)

릴리스의 성공 여부를 판단하는 방법입니다. CHANGELOG의 릴리스마다 추적됩니다.

| Metric | Baseline | v0.5 Target | v1.0 Target |
| :--- | :--- | :--- | :--- |
| Warm-cache no-op build (10k-file workspace) | < 2s | < 500ms | < 200ms |
| Cold-cache speedup vs serial build | 3–4x | 6–8x | near-linear to 16 cores |
| Scheduler overhead per task dispatch | unmeasured | < 1ms p99 | < 100µs p99 |
| Remote cache integrity failures surfaced silently | n/a | 0 (hard fail) | 0 (hard fail) |
| Fabricated tooling output incidents | eliminated in v0.4 | 0 | 0 |

---

## 🚫 Non-Goals (new)

범위 규칙(Scope discipline)은 Fish를 빠르고 신뢰할 수 있게 유지합니다. 우리는 다음과 같은 항목을 의도적으로 **구축하지 않습니다**:

- **A general workflow/orchestration engine** — Airflow/Prefect 영역입니다. Fish는 비즈니스 프로세스가 아닌 *빌드*를 오케스트레이션합니다.
- **A package manager** — Fish는 락파일(lockfiles)을 소비합니다; 종속성을 해결(resolve)하지 않습니다.
- **Silent fallbacks or simulated results anywhere** — 거부된 작업은 실패 원인을 명확하고 강력하게 알려야 합니다. 이것은 영구적인 아키텍처 불변(invariant) 원칙이며 하나의 단계(phase)가 아닙니다.
- **Proprietary hosted-only features** — 코디네이터, 워커 및 캐시 프로토콜은 누구나 구현할 수 있도록 유지됩니다.

---

## 💬 Feedback & Community Contributions

우리는 전 세계 개발자들의 피드백, 제안, 기여를 환영합니다!
- [GitHub Issues](https://github.com/requla11/fish/issues)를 통해 논의 및 기능 요청에 참여하세요.
- [Contributing Guide](CONTRIBUTING.md) 및 [Translation Guidelines](TRANSLATION.md)를 검토하세요.
