# Fish 아키텍처

> 🌐 **번역 및 기여:** 이 문서를 모국어로 번역하거나 개선하고 싶으신가요? [번역 가이드라인](TRANSLATION.md)을 참조하세요.

이 문서는 fish 빌드 오케스트레이션 시스템의 고수준 아키텍처를 설명합니다.

## 개요

Fish는 모노레포 및 다국어(polyglot) 프로젝트를 위해 설계된 캐시 우선(cache-first), 다국어 빌드 오케스트레이션 시스템입니다. 종속성 그래프, 병렬 스케줄러, 실행기 및 CAS 아티팩트 캐시를 사용하여 빌드 성능을 최적화합니다.

## 핵심 컴포넌트

### 1. 작업 공간 탐색 (`fish-core`)

**목적**: 프로젝트 구조 탐색 및 모델링

**역할 및 책임**:
- 패키지/프로젝트에 대한 작업 공간 스캔
- 매니페스트 파일을 기반으로 프로젝트 유형 감지
- 마이크로 글롭(micro-globs)을 통한 입력 파일 필터링 (`MicroInputFilter`)
- 패키지 간 종속성 그래프 빌드
- IDE 컴파일 데이터베이스 생성 (`CompilationDatabase`, `compile_commands.json`)
- 밀폐형(hermetic) 컴파일러 툴체인 관리 및 격리 (`ToolchainRegistry`, `ToolchainSpec`)
- 패키지 메타데이터 관리

**주요 타입**:
- `Package`: 단일 패키지/프로젝트를 나타냄
- `Workspace`: 종속성이 있는 패키지 모음
- `Manifest`: 프로젝트 구성 (Cargo.toml, package.json 등)
- `MicroInputFilter`: 세밀한 글롭 매처 및 파일 필터
- `CompilationDatabase`: 표준 컴파일 명령 데이터베이스
- `ToolchainRegistry`: 밀폐형 툴체인 구성 관리자

### 2. 빌드 그래프 (`fish-graph`)

**목적**: 빌드 종속성, 실행 순서 및 대수적 쿼리 모델링

**역할 및 책임**:
- 빌드 작업의 방향성 비순환 그래프(DAG) 생성
- 실행 순서를 위한 위상 정렬(topological sort) 계산
- 다국어 모노레포를 위한 하위 그래프 병합 (`merge_subgraph`)
- 런타임 실행 중 동적 노드 확장 (`DynamicGraphExpander`)
- 작업 상태 추적 (pending, running, completed, failed)
- 대수적 쿼리 평가 (`GraphQueryEngine`에서 `deps()`, `rdeps()`, `allpaths()`, `somepath()`, `filter()`, `union()`, `intersect()`, `except()` 지원)
- 순환 종속성 감지

**주요 타입**:
- `BuildGraph`: 작업의 방향성 비순환 그래프
- `Node`: 개별 빌드 작업
- `NodeId`: 그래프 구조에 대한 타입 안전(type-safe) 인덱스
- `DynamicGraphExpander`: 동적 하위 작업 생성기
- `GraphQueryEngine`: 그래프 쿼리 표현식 평가기
- `QueryExpr`: 대수적 쿼리 AST

### 3. 실행기 (`fish-executor`)

**목적**: 빌드 명령 실행, 프로세스 관리 및 파일 시스템 복제 처리

**역할 및 책임**:
- 빌드 프로세스 생성 및 관리
- stdout/stderr 캡처
- 프로세스 시간 초과 및 취소 처리
- 기록 중 복사(copy-on-write) 익스텐트 및 하드링크를 사용한 빠른 파일 시스템 복제 (`KernelCowCloner`)
- 빠른 링커 자동 감지 및 플래그 합성 (`mold`, `lld`, `msvc`를 지원하는 `LinkerDispatcher`)
- 인수가 OS 제한을 초과할 때 자동 응답 파일 합성 (`@fish_args.rsp`)
- 확장 가능한 작업 미들웨어 파이프라인 (`TaskMiddleware`, `TurboLinker`, `SuperOptimizer`)
- 실행 결과 반환

**주요 타입**:
- `CommandSpec`: 환경이 포함된 명령 사양
- `AsyncExecutor`: 논블로킹 프로세스 실행 엔진
- `KernelCowCloner`: 기록 중 복사 및 빠른 복제기
- `LinkerDispatcher`: 최신 링커 감지기
- `ResponseFileWriter`: 인수 파일 합성기
- `TaskMiddleware`: 작업 가로채기를 위한 미들웨어 트레이트(trait)
- `ExecutionResult`: 명령 실행 결과

### 4. 스케줄러 (`fish-scheduler`)

**목적**: 병렬, 투기적(speculative) 및 분산 실행을 위한 작업 스케줄링

**역할 및 책임**:
- 사용 가능한 작업의 준비 대기열 유지
- 사용 가능한 워커 간에 작업 분배
- 시스템 메모리 압박을 모니터링하고 동시성을 스로틀링하는 커널 리소스 거버너 (`KernelResourceGovernor`)
- 메타데이터 준비 시 다운스트림 컴파일 차단을 해제하는 컴파일러 파이프라이닝 조정 (`PipelinedCompilationCoordinator`)
- 컴파일러 전반의 글로벌 스레드 토큰 관리를 위한 GNU Jobserver 풀 통합 (`JobserverPool`)
- 동적 원격 레이싱 (`DynamicRacingExecutor`): 동시 로컬 vs 원격 실행
- LPT(Longest Processing Time) 스케줄링을 사용한 분산 작업 실행(DTE) 빈 패킹 (`DteBinPacker`)
- 더티 노드 무효화 및 핫 그래프 캐시 사전 워밍을 지원하는 실시간 파일 시스템 감시자 데몬 (`FsWatcherDaemon`)
- 작업 종속성 준수
- 작업 완료 및 실패 처리

**주요 타입**:
- `Scheduler`: 작업 스케줄링 엔진
- `KernelResourceGovernor`: 메모리 압박 모니터
- `PipelinedCompilationCoordinator`: 파이프라이닝 단계 관리자
- `JobserverPool`: 글로벌 토큰 기반 동시성 풀
- `DynamicRacingExecutor`: 로컬 vs 원격 레이서
- `DteBinPacker`: 균형 잡힌 다중 에이전트 CI 파티셔너
- `FsWatcherDaemon`: 실시간 변경 리스너 및 더티 노드 추적기
- `WorkStealingPool`: 락프리(Lock-free) 작업 분배기

### 5. 캐시 (`fish-cache`)

**목적**: 증분 빌드를 위한 지문(fingerprint) 기반 캐싱

**역할 및 책임**:
- 파일 콘텐츠 지문 계산 (Blake3)
- 실행 결과 캐싱
- 캐시 유효성 판단
- 캐시 무효화 지원

**주요 타입**:
- `Fingerprint`: 메타데이터가 포함된 콘텐츠 해시
- `CacheEntry`: 캐시된 실행 결과
- `FileLevelCache`: 파일 수준 캐싱 전략

### 6. CAS 엔진 (`fish-cas`)

**목적**: 아티팩트 캐싱을 위한 콘텐츠 주소 지정 스토리지 (Content-Addressable Storage)

**역할 및 책임**:
- 콘텐츠 해시별로 아티팩트 저장
- 로컬 및 원격 스토리지 지원
- 아티팩트 압축 (Zstandard)
- 중복 제거 제공

**주요 타입**:
- `ArtifactStore`: 아티팩트 스토리지 인터페이스
- `LocalStorage`: 로컬 파일 시스템 스토리지
- `RemoteStorage`: 원격 스토리지 (S3, GCS, MinIO)

### 7. 원격 캐시 (`fish-remote-cache`)

**목적**: 계층화된 L1/L2 복합 캐싱

**역할 및 책임**:
- 빠른 액세스를 위한 로컬 L1 캐시
- 공유를 위한 원격 L2 캐시
- 캐시 채우기 및 축출(eviction)
- 캐시 적중/실패 추적

**주요 타입**:
- `CompositeCache`: 계층화된 캐시 구현
- `CachePolicy`: 캐시 채우기 및 축출 정책

### 8. 워커 (`fish-worker`)

**목적**: 분산 빌드 실행

**역할 및 책임**:
- 원격 워커 탐색 및 등록
- 워커 간 작업 분배
- 결과 수집 및 집계
- 주문형(on-demand) 파일 액세스를 위한 가상 파일 시스템

**주요 타입**:
- `WorkerServer`: 워커 데몬
- `ClusterExecutor`: 클러스터 작업 실행
- `VirtualFileSystem`: 인메모리 VFS

### 9. 샌드박싱 (`fish-sandbox`)

**목적**: 밀폐형(Hermetic) 환경 격리

**역할 및 책임**:
- 빌드 환경 격리
- 파일 시스템 액세스 제어
- 네트워크 격리
- 리소스 제한

**주요 타입**:
- `Sandbox`: 샌드박스 구현
- `SandboxConfig`: 샌드박스 구성

### 10. 플러그인 시스템 (`fish-plugin`)

**목적**: 확장 가능한 규칙 시스템

**역할 및 책임**:
- 사용자 정의 플러그인 로드
- 스크립트 플러그인 실행 (Shell, Python, Node, WASM, Lua)
- 플러그인 탐색 및 관리
- 플러그인 API

**주요 타입**:
- `PluginManager`: 플러그인 관리자
- `ScriptPlugin`: 스크립트 기반 플러그인
- `PluginExecutor`: 플러그인 실행 엔진

## 언어 백엔드

모든 백엔드는 `fish-backend-api` 크레이트의 균일한 `EcosystemBackend` 컨트랙트를 구현하며 자신을 `fish-cli/src/backend_registry.rs`에 등록합니다 — 생태계를 추가한다는 것은 해당 트레이트를 구현하고 레지스트리에 한 줄을 추가하는 것을 의미합니다.

### 백엔드 인터페이스

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

### 지원되는 백엔드

- **Rust** (`fish-backend-rust`): Cargo workspaces
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

## 보안 기능

### 1. 아티팩트 서명 (`fish-security` / `fish-remote-cache`)

- `FISH_SIGNING_SEED`를 통한 Ed25519 서명; 공개 키는 `fish signing-key`로 내보냄
- SLSA/in-toto 출처(provenance) 구문 (`fish-security/src/slsa.rs`)
- 원격 캐시 서명 게이트는 `FISH_TRUSTED_KEYS`에 대해 모든 다운로드를 확인합니다 (`fish-remote-cache/signature_gate.rs`)
- 전체 프로듀서/컨슈머 흐름은 `docs/signing.md` 참조

### 2. 보안 스캐너 (`fish-security`)

- 종속성 취약점 스캐닝
- 다중 백엔드 지원
- 심각도 기반 차단
- CVSS 점수 추적

## CI/CD 생성

### CI 생성기 (`fish-ci-generator`)

여러 CI/CD 플랫폼을 지원합니다:
- GitHub Actions
- GitLab CI
- CircleCI
- Bitbucket Pipelines

### 매트릭스 생성

- 다중 플랫폼 지원 (Linux, macOS, Windows)
- 다중 아키텍처 (x86_64, ARM64)
- 버전 매트릭스 (Rust, Node 등)
- 종속성 기반 최적화

## 고급 기능

### 1. 빌드 분석 (`fish-analytics`)

- 실시간 캐시 적중률 추적
- 빌드 지표 수집
- 성능 시각화
- 최적화 제안

### 2. 증분 분석 (`fish-incremental`)

- Rust, TypeScript/JavaScript, Python, Go를 위한 AST 기반 종속성 추론 (`DependencyInferenceEngine`)
- 정확한 소스 파일 수정 또는 해시 불일치를 식별하는 더티(dirty) 리빌드 진단 (`DirtyExplainer`, `fish build --explain`)
- 빌드 패턴 감지 및 핫스팟 식별
- 리팩터링 제안 및 리빌드 빈도 분석

### 3. 빌드 데몬 & IPC (`fish-cli::daemon`)

- 개행으로 구분된 메시지를 통해 JSON-RPC 2.0을 사용하는 백그라운드 데몬 (`FishDaemon`)
- 전송(Transport): Unix의 경우 Unix 도메인 소켓, Windows의 경우 `127.0.0.1`에서 TCP 사용
- 포트 구성 가능: `fish daemon start --port <PORT>` (기본값 `9527`)
- 반복 호출을 위한 웜 그래프(Warm graph) 캐싱
- 명령어: `fish daemon start`, `fish daemon status`, `fish daemon stop`

### 4. 프로필 기반 최적화 (Profile-Guided Optimization, `fish-cli::pgo`)

- 2단계 LLVM PGO 워크플로 오케스트레이션 (`PgoManager`)
- 자동화된 `-Cprofile-generate` 계측 및 `llvm-profdata merge`
- 최대 런타임 성능을 위한 `-Cprofile-use`를 사용한 재컴파일

### 5. 작업 파이프라인 토폴로지 (`fish-cli::pipeline`)

- `fish.toml`을 통해 구성된 Turborepo/Nx 스타일의 위상 작업 파이프라인
- 패키지 간 종속성 규칙 (예: 종속성 출력이 먼저 빌드되도록 보장하는 `^build`)
- 구성 가능한 환경 변수 및 입력 파일 지문 해시

### 계획된 크레이트 (아직 작업 공간에 없음)

다음 크레이트는 초기 초안에 설명되어 있지만 아직 작업 공간에 존재하지 않습니다. 여기에는 로드맵 항목으로만 나열되어 있습니다:

- `fish-multiplatform` — 플랫폼 감지, 대상 트리플(target triples), CI 매트릭스
- `fish-notifications` — Slack/Discord/이메일 빌드 알림
- `fish-flaky-detection` — 통계적 불안정(flaky) 테스트 감지 및 재시도 정책
- `fish-docker-builder` — 일급(first-class) Docker 아티팩트 및 계층 캐싱 (현재 Docker 오케스트레이션은 `fish-backend-docker`에 위치)
- `fish-templates` — 공유 가능한 파이프라인 템플릿 (Handlebars 렌더링)

## 벤더링된(Vendored) 서브모듈

두 개의 동반 프로젝트가 git 서브모듈로 벤더링되었으며 작업 공간의 구성원입니다:

- **`submodules/apple`** — 밀폐형 샌드박스 및 프로세스 격리 데몬 (커널 수준 샌드박싱, CoW 스토리지 감옥, SLSA/SPDX/CycloneDX 출처). Apple Inc.와 무관한 독립 프로젝트입니다.
- **`submodules/banana`** — Fish를 위한 배포, P2P 스웜(swarm) 및 공급망 인프라 동반 프로젝트.

## 데이터 흐름

### 빌드 실행 흐름

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

### 분산 빌드 흐름

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

## 성능 최적화

### 1. 레벨 파티셔닝(Level Partitioning)

빌드 수준별로 독립적인 패키지를 단일 툴체인 호출로 그룹화하여 프로세스 생성 오버헤드를 제거합니다.

### 2. 캐시 우선(Cache-First) 실행

지문 기반 캐싱은 입력이 변경되지 않았을 때 즉각적인 리빌드를 가능하게 합니다.

### 3. 병렬 실행

작업은 종속성을 준수하면서 병렬로 실행되어 CPU 활용도를 극대화합니다.

### 4. 증분 빌드(Incremental Builds)

종속성 그래프 변경에 따라 영향을 받는 패키지만 리빌드합니다.

### 5. 분산 실행

원격 워커는 대규모 프로젝트를 위한 수평적 확장(horizontal scaling)을 가능하게 합니다.

## 아키텍처 상태

Fish는 단일 언어 Rust 작업 공간입니다. 이 저장소에는 Python이나 Go 서비스가 없으며, 현재 어떤 크레이트도 gRPC/protobuf를 사용하지 않습니다. 이 문서의 초기 초안에서는 "Tri-Engine" 아키텍처를 설명했지만, 해당 설명은 코드베이스와 일치하지 않아 제거되었습니다.

### 현재 코어 (구현됨)

- **`fish-core`**: 작업 공간 탐색, 매니페스트 모델, 세밀한 입력 필터링.
- **`fish-graph`**: 종속성 그래프, 위상 정렬, 대수적 쿼리 평가 (`deps`, `rdeps`, `somepath`).
- **`fish-executor`**: 프로세스 실행, 미들웨어 체인, 응답 파일 생성.
- **`fish-scheduler`**: GNU Jobserver 풀, 작업 훔치기(work-stealing), 병렬 실행.
- **`fish-cache`**: Blake3를 사용한 다계층 지문 생성 및 2단계 가지치기(pruning).
- **`fish-cas`**: ZSTD 압축을 지원하는 콘텐츠 주소 지정 아티팩트 스토리지.
- **`fish-cli`**: ratatui 및 clap 기반의 터미널 사용자 인터페이스.

### 계획됨: 교차 언어 컨트랙트 (`proto/`)

`proto/fish/v1/` 아래의 파일들 (`build.proto`, `ai.proto`, `coordinator.proto`)은 미래 지향적인 인터페이스 초안일 뿐입니다. 이 파일들은 아직 컴파일되거나 어떤 크레이트에서도 참조되지 않습니다 — 작업 공간에는 `prost`/`tonic` 종속성이 없습니다. 현재 배포된 분산 기능들은 대신 일반 HTTP/JSON을 사용합니다 (`crates/fish-worker` 및 `crates/fish-remote-cache` 참조).

## 보안 고려 사항

- 보안에 민감한 크레이트에는 unsafe 코드 없음
- 모든 백엔드에 걸친 입력 유효성 검사
- 모든 작업에 대한 최소 권한(Least privilege) 원칙
- 보안 작업을 위한 감사 로깅
- 안전한 비밀(secret) 관리
- Ed25519 아티팩트 서명 및 암호화된 SBOM 생성

