<div align="center">

<img src="docs/public/logo.png" alt="Fish Logo" width="180" />

# 🐟 Fish

**다국어 모노레포를 위한 초고속, 캐시 우선 빌드 오케스트레이션 시스템**

[![CI](https://github.com/requla11/fish/actions/workflows/dogfood.yaml/badge.svg)](https://github.com/requla11/fish/actions/workflows/dogfood.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/requla11/fish)

[English](README.md) | [Français](README.fr.md) | [Deutsch](README.de.md) | [日本語](README.ja.md) | [한국어](README.ko.md) | [Tiếng Việt](README.vi.md) | [简体中文](README.zh-hans.md) | [繁體中文](README.zh-hant.md)

</div>

---

**Fish**는 **Rust 2024**로 설계된 고성능 빌드 오케스트레이션 엔진입니다. Turborepo의 속도와 단순함을 Bazel의 다국어 처리 능력과 함께 제공하며, **Starlark과 같은 복잡한 구성 언어나 커스텀 빌드 DSL을 요구하지 않습니다**.

Fish는 자동으로 툴체인을 검색하고, 소스 트리를 분석하여 언어 간 종속성 엣지(edge)를 추론하며, 락 프리(lock-free) 작업 훔치기(work-stealing) 풀에서 작업을 스케줄링하고, 암호학적으로 안전한 **BLAKE3** 콘텐츠 주소 지정 스토리지(CAS) 및 **Zstandard** 압축을 사용하여 모든 아티팩트를 캐시합니다.

> 💡 **알림:** Fish는 기존 컴파일러 및 패키지 관리자(Cargo, Go, npm/pnpm, Python, Clang 등)를 조율(coordinate)합니다. 이들을 대체하지 않습니다. [fish-shell](https://fishshell.com)과는 무관하며 이름만 공유합니다.

---

## ✨ 주요 특징

| 기능 | 설명 |
| :--- | :--- |
| ⚡ **서브 밀리초 스케줄링** | Chase-Lev 작업 훔치기 큐와 크리티컬 패스(critical-path) 스케줄링으로 100µs 미만으로 작업을 디스패치합니다. |
| 🌐 **11개 이상의 언어 생태계** | Rust, Go, TypeScript/JS, Python, C/C++, Java, .NET, Swift, Dart, Zig 및 Docker를 위한 네이티브 백엔드. |
| 🔗 **자동 종속성 추론** | 계약 우선(Contract-first) 교차 언어 링킹: 참조(`include_str!`, JSON import 등)가 수동 `depends_on` 없이 자동으로 DAG 엣지를 연결합니다. |
| 💾 **고처리량 CAS 캐시** | 계층화된 L1/L2 캐싱 및 ZSTD 압축을 갖춘 중복 제거된 BLAKE3 콘텐츠 주소 지정 스토리지. |
| 📡 **무설정 P2P 캐시** | 클라우드 서버 비용 없이 로컬 Wi-Fi / LAN을 통해 팀원들과 빌드 아티팩트를 P2P로 공유합니다. |
| 🛡️ **밀폐된(Hermetic) 격리** | 멀티 플랫폼 샌드박싱: Linux namespaces & Landlock, macOS seatbelt 및 Windows 보안 토큰. |
| 📊 **실시간 인터랙티브 UI** | 대화형 SVG DAG 시각화 도구 및 원격 측정(telemetry) 그래프를 제공하는 내장 웹 대시보드(`fish ui`). |

---

## 🚀 빠른 설치

### 1줄 설치기

#### Linux 및 macOS
```bash
curl -fsSL https://raw.githubusercontent.com/requla11/fish/main/scripts/install.sh | sh
```

#### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/requla11/fish/main/scripts/install.ps1 | iex
```

---

### 패키지 관리자

| 플랫폼 | 패키지 관리자 | 명령어 |
| :--- | :--- | :--- |
| **Windows** | **Scoop** | `scoop install https://raw.githubusercontent.com/requla11/fish/main/packaging/fish.json` |
| **Windows** | **Winget** | `winget install requla11.fish` |
| **macOS** | **Homebrew** | `brew tap requla11/fish https://github.com/requla11/homebrew-fish && brew install fish` |
| **Cargo** | **crates.io / Git** | `cargo install --git https://github.com/requla11/fish.git fish-cli` |

---

## 🏁 빠른 시작

다국어 리포지토리로 이동하여 다음을 실행하세요:

```bash
# 스마트 캐싱을 사용하여 전체 작업 공간을 병렬로 빌드합니다.
fish build

# 모든 언어에 걸쳐 모든 테스트 스위트를 실행합니다.
fish test

# 감시(Watch) 모드: 파일 변경 시 다시 컴파일하고 다시 테스트합니다.
fish dev

# 빌드 아티팩트를 정리합니다. (--all을 사용하면 로컬 캐시를 포함한 모든 것을 정리합니다)
fish clean --all

# 인터랙티브 웹 대시보드 및 DAG 시각화 도구를 실행합니다.
fish ui --open
```

### 다국어 데모 사용해보기

**Rust + Go + Python + TypeScript**를 결합한 현실적인 계약 우선 모노레포가 포함되어 있습니다:

```bash
cd examples/polyglot-demo
fish build
fish graph --format tree
```

출력 예시:
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

## 🛠️ 지원되는 생태계

Fish는 기본적으로 11개의 주요 생태계에 걸쳐 프로젝트를 감지하고 오케스트레이션합니다:

| 생태계 | 감지되는 매니페스트 | 기본 작업(Tasks) |
| :--- | :--- | :--- |
| **Rust** | `Cargo.toml` | `cargo check`, `cargo build`, `cargo test` |
| **TypeScript / Node** | `package.json`, `tsconfig.json` | `typecheck`, `build`, `test` |
| **Go** | `go.mod` | `go vet`, `go build`, `go test` |
| **Python** | `pyproject.toml`, `requirements.txt` | 구문 컴파일, `pytest`, 린트(lint) |
| **C / C++** | `CMakeLists.txt`, `fish.cc.json` | CMake configure, build, `ctest` |
| **Java** | `pom.xml`, `build.gradle` | compile, test |
| **.NET / C#** | `*.csproj`, `*.sln` | `dotnet build`, `dotnet test` |
| **Swift** | `Package.swift` | `swift build`, `swift test` |
| **Dart / Flutter** | `pubspec.yaml` | `dart analyze`, `dart test` |
| **Zig** | `build.zig` | `zig build`, `zig test` |
| **Docker / OCI** | `Dockerfile`, `docker-compose.yml` | 멀티 스테이지 이미지 빌드, OCI 컴파일 |

---

## 📋 필수 명령어

Fish는 CLI를 깔끔하고 직관적이며 개발자 친화적으로 유지합니다:

```text
빌드 및 테스트:
  fish build             프로젝트 그래프에서 검색된 모든 대상을 빌드합니다.
  fish check             링킹 없이 대상을 타입 체크하고 검증합니다.
  fish test              작업 공간 전체의 모든 테스트 스위트를 실행합니다.
  fish run [TARGET]      특정 바이너리 대상을 빌드하고 실행합니다.
  fish dev (or watch)    파일을 지속적으로 감시하고 점진적 리빌드를 트리거합니다.

검사 및 이해:
  fish graph             DAG를 스테이지 트리, DOT 또는 JSON으로 시각화합니다.
  fish why <QUERY>       대상이 리빌드된 이유를 자연어로 질문합니다.
  fish ui                실시간 웹 대시보드 및 인터랙티브 DAG 시각화 도구를 엽니다.
  fish doctor            설치된 툴체인, 캐시 무결성 및 환경을 진단합니다.

유지 관리 및 정리:
  fish clean             프로젝트 빌드 대상을 제거합니다. (-a/--all을 전달하여 ~/.fish/cache를 지웁니다)
  fish fix               AI 및 컴파일러 기반 오류 진단 및 자동 수정.
  fish ci init           최적화된 CI/CD 워크플로를 생성합니다. (GitHub Actions, GitLab 등)
  fish affected          git 변경 사항의 영향을 받는 패키지만 빌드하거나 테스트합니다.
```

---

## 🏗️ 아키텍처 및 작업 공간 레이아웃

이 엔진은 엄격한 경계 격리를 유지하는 모듈식 Rust 작업 공간(28개의 크레이트)으로 구성되어 있습니다:

```text
crates/
  fish-core/         작업 공간 검색, 매니페스트 모델 및 DAG 병합기
  fish-graph/        종속성 그래프, 위상 정렬 및 쿼리 대수
  fish-executor/     프로세스 실행, 미들웨어 체인 및 응답 파일
  fish-scheduler/    병렬 작업 훔치기 스케줄러, GNU 작업 서버 풀, 레이싱 및 DTE
  fish-cache/        지문(Fingerprint) 캐시, 2단계 정리 및 형태적(morphic) 해시
  fish-cas/          BLAKE3 + ZSTD 압축을 사용한 콘텐츠 주소 지정 아티팩트 스토리지
  fish-incremental/  변경 감지, AST 추론 및 더티 리빌드 설명기
  fish-backend-*/    EcosystemBackend를 구현하는 11개의 언어 및 툴체인 어댑터
  fish-worker/       분산 실행 서버 및 스트리밍 VFS 프로토콜
  fish-remote-cache/ Ed25519 서명 게이팅을 갖춘 고처리량 원격 캐시 서버
  fish-security/     다중 계층 보안, OSV 취약성 스캐너 및 SLSA 출처(provenance)
  fish-cli/          통합 명령줄 애플리케이션, 데몬 IPC 및 터미널 렌더링
submodules/          벤더링된 컴패니언 격리 엔진:
  apple/             밀폐된 샌드박스 및 OS 프로세스 격리 데몬
  banana/            P2P 스웜 메시, OCI 컨테이너 빌더 및 머클 원장
examples/            실행 준비가 된 다국어 모노레포 데모
```

---

## 🌿 브랜치 정책

Fish는 엄격한 브랜치 수명 주기를 따릅니다:

```text
dev (활발한 개발, 테스트, 기능 추가)
  ↓
  ↓ 검증: cargo test --workspace & cargo clippy
  ↓
main (안정적인 프로덕션 준비 완료 릴리스)
```

- **`dev`** — 모든 활발한 작업, 기능 브랜치 및 풀 리퀘스트(PR)는 이곳으로 모입니다.
- **`main`** — 안정적인 태그 릴리스 전용입니다.

---

## 🧪 개발 및 검증

로컬에서 코드베이스를 검증하려면:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

---

## 📖 문서 및 커뮤니티

- [아키텍처 가이드](ARCHITECTURE.md) — 심층적인 아키텍처 설계 및 컴포넌트.
- [개발 환경 설정](DEVELOPMENT.md) — 로컬 설정, 디버깅 및 벤치마크.
- [기여 가이드라인](CONTRIBUTING.md) — 변경 사항 제안 및 백엔드 추가 방법.
- [AI 에이전트 워크플로](docs/AI_AGENT_WORKFLOW.md) — AI 코딩 에이전트를 위한 모범 사례.

---

## 📄 라이선스 및 면책 조항

Fish는 [MIT 라이선스](LICENSE)에 따라 라이선스가 부여됩니다.

> **면책 조항:** 이 프로젝트는 독립적인 빌드 오케스트레이션 시스템입니다. 이름에 "fish"를 사용하는 기타 관련 없는 도구, 패키지 또는 프로젝트(`fish-shell`, `fish-image` 등)는 독립적이며 Fish 빌드 오케스트레이션 프로젝트와 제휴, 후원 또는 보증되지 않습니다.
