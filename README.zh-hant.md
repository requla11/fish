<div align="center">

<img src="docs/public/logo.png" alt="Fish Logo" width="180" />

# 🐟 Fish

**極速、快取優先的多語言 Monorepo 構建編排與加速系統**

[![CI](https://github.com/requla11/fish/actions/workflows/dogfood.yaml/badge.svg)](https://github.com/requla11/fish/actions/workflows/dogfood.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/requla11/fish)

[English](README.md) • [Tiếng Việt](README.vi.md) • [简体中文](README.zh-hans.md) • [繁體中文](README.zh-hant.md) • [日本語](README.ja.md)

</div>

---

**Fish** 是一款採用 **Rust 2024** 精心打造的高效能多語言構建編排引擎。它兼具 Turborepo 的極速與直觀體驗，以及 Bazel 強大的多語言處理能力 — **完全無需學習 Starlark 或複雜的自定義構建 DSL**。

Fish 能夠自動發現工具鏈、解析原始碼樹以智慧推導跨語言依賴邊、利用無鎖工作竊取（Work-Stealing）池調度任務，並基於高強度 **BLAKE3** 內容定址儲存（CAS）與 **Zstandard** 演算法實現全產物快取。

> 💡 **提示：** Fish 用於協調並調度現有的編譯器和套件管理器（Cargo、Go、npm/pnpm、Python、Clang 等），並非替代品。本專案與互動式 Shell [fish-shell](https://fishshell.com) 無任何關聯。

---

## ✨ 核心特性亮點

| 功能 | 詳細描述 |
| :--- | :--- |
| ⚡ **亞毫秒級高效調度** | 基於 Chase-Lev 工作竊取佇列與關鍵路徑演算法，任務調度分發延遲低於 100µs。 |
| 🌐 **支援 11+ 語言生態** | 原生支援 Rust、Go、TypeScript/JS、Python、C/C++、Java、.NET、Swift、Dart、Zig 與 Docker。 |
| 🔗 **跨語言依賴自動推導** | 契約優先（Contract-first）：引用關係（如 `include_str!`、JSON 匯入）自動構建 DAG 邊，無需手寫 `depends_on`。 |
| 💾 **高吞吐 CAS 快取** | 基於 BLAKE3 去重的內容定址儲存，結合 L1/L2 分層快取與 ZSTD 快速壓縮。 |
| 📡 **零配置 P2P 區域網路快取** | 支援團隊成員在本地 Wi-Fi / 區域網路內點對點秒級同步構建產物，無需雲端伺服器費用。 |
| 🛡️ **密封沙盒隔離** | 多平台沙盒機制：Linux namespaces & Landlock、macOS seatbelt 與 Windows 安全權杖。 |
| 📊 **即時互動式 Web 控制台** | 內建互動式 Web 儀表板（`fish ui`），提供即時 SVG DAG 依賴圖與效能指標遙測。 |

---

## 🚀 快速安裝

### 一鍵腳本安裝

#### Linux & macOS
```bash
curl -fsSL https://raw.githubusercontent.com/requla11/fish/main/scripts/install.sh | sh
```

#### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/requla11/fish/main/scripts/install.ps1 | iex
```

---

### 套件管理器安裝

| 作業系統 | 套件管理器 | 命令 |
| :--- | :--- | :--- |
| **Windows** | **Scoop** | `scoop install https://raw.githubusercontent.com/requla11/fish/main/packaging/fish.json` |
| **Windows** | **Winget** | `winget install requla11.fish` |
| **macOS** | **Homebrew** | `brew tap requla11/fish https://github.com/requla11/homebrew-fish && brew install fish` |
| **Cargo** | **crates.io / Git** | `cargo install --git https://github.com/requla11/fish.git fish-cli` |

---

## 🏁 快速上手

在任意多語言專案的根目錄下運行：

```bash
# 平行構建整個工作區並啟用智慧快取
fish build

# 運行所有語言的測試套件
fish test

# 監聽模式：在檔案發生變動時自動增量構建與測試
fish dev

# 清理構建產物（添加 --all 可徹底清空本地快取 ~/.fish/cache）
fish clean --all

# 打開即時互動式 Web 控制台與 DAG 依賴視覺化
fish ui --open
```

### 體驗多語言範例專案

Fish 自帶一個融合了 **Rust + Go + Python + TypeScript** 的契約優先 Monorepo 範例：

```bash
cd examples/polyglot-demo
fish build
fish graph --format tree
```

構建輸出範例：
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

## 🛠️ 支援的語言生態系統

Fish 能夠原生識別並編排以下 11 個主流開發生態：

| 語言生態 | 識別清單檔案 | 預設執行任務 |
| :--- | :--- | :--- |
| **Rust** | `Cargo.toml` | `cargo check`, `cargo build`, `cargo test` |
| **TypeScript / Node** | `package.json`, `tsconfig.json` | `typecheck`, `build`, `test` |
| **Go** | `go.mod` | `go vet`, `go build`, `go test` |
| **Python** | `pyproject.toml`, `requirements.txt` | 語法檢查, `pytest`, 程式碼檢查 |
| **C / C++** | `CMakeLists.txt`, `fish.cc.json` | CMake 設定, 構建, `ctest` |
| **Java** | `pom.xml`, `build.gradle` | 編譯, 測試 |
| **.NET / C#** | `*.csproj`, `*.sln` | `dotnet build`, `dotnet test` |
| **Swift** | `Package.swift` | `swift build`, `swift test` |
| **Dart / Flutter** | `pubspec.yaml` | `dart analyze`, `dart test` |
| **Zig** | `build.zig` | `zig build`, `zig test` |
| **Docker / OCI** | `Dockerfile`, `docker-compose.yml` | 多階段映像構建, OCI 打包 |

---

## 📋 常用 CLI 命令速查

Fish 保持命令列工具簡單、直觀且易用：

```text
構建與測試：
  fish build             構建專案圖中識別出的所有目標
  fish check             極速型別與語法檢查（不連結二進位檔）
  fish test              執行工作區內的所有單元與整合測試
  fish run [TARGET]      構建並直接運行指定的二進位目標
  fish dev (或 watch)    監聽原始碼改動並自動觸發增量重構

分析與觀察：
  fish graph             以樹狀圖、DOT 或 JSON 形式列印 DAG 依賴圖
  fish why <QUERY>       使用自然語言詢問特定目標被重新構建的原因
  fish ui                打開即時 Web 控制台與互動式 DAG 圖
  fish doctor            全面診斷本地工具鏈就緒情況、快取與環境設定

清理與維護：
  fish clean             清理當前專案構建產物（帶 -a/--all 徹底刪除 ~/.fish/cache）
  fish fix               基於 AI 與編譯器反饋的智慧錯誤診斷與自動修復
  fish ci init           快速生成優化的 CI/CD 設定（GitHub Actions, GitLab 等）
  fish affected          僅針對 Git 變更所影響的相關套件進行構建或測試
```

---

## 🏗️ 架構設計與工作區模組劃分

本專案採用嚴謹的模組化 Rust 工作區結構（共 28 個 Crates）：

```text
crates/
  fish-core/         專案自動探測、清單解析與 DAG 合併器
  fish-graph/        依賴有向無環圖、拓撲排序與代數查詢引擎
  fish-executor/     底層處理序執行、中介軟體鏈與回應參數檔案支援
  fish-scheduler/    平行工作竊取調度器、GNU Jobserver 池與動態競速
  fish-cache/        多層指紋快取、雙階段修剪與同態雜湊
  fish-cas/          BLAKE3 + ZSTD 高效能內容定址構件儲存
  fish-incremental/  原始碼變動擷取、AST 依賴推導與構建診斷說明
  fish-backend-*/    11 個主流語言與工具鏈適配層（實現 EcosystemBackend）
  fish-worker/       分散式遠端執行節點與串流虛擬檔案系統（VFS）
  fish-remote-cache/ 支援 Ed25519 簽名驗證的高吞吐遠端快取伺服器
  fish-security/     多層次安全合規、OSV 漏洞掃描與 SLSA 產物簽名認證
  fish-cli/          統一定義命令列互動介面、守護處理序 IPC 與終端互動呈現
submodules/          配套的安全與網路子系統：
  apple/             Hermetic 密封沙盒與系統處理序安全隔離守護處理序
  banana/            P2P Swarm 區域網路、OCI 容器構建器與 Merkle 帳本
examples/            現成可運行的多語言 Monorepo 實戰範例
```

---

## 🌿 分支開發規範（Branch Policy）

Fish 嚴格遵循雙主分支工作流程：

```text
dev（活躍特性開發、日常測試、Bug 修復）
  ↓
  ↓ 嚴格驗證：cargo test --workspace & cargo clippy
  ↓
main（生產就緒的高品質穩定發布版本）
```

- **`dev`** — 所有新功能開發、試驗性程式碼與 Pull Request 均合併至此分支。
- **`main`** — 僅包含經過嚴格驗證的高穩定性正式 Release 程式碼。

---

## 🧪 驗證與本地測試

提交程式碼前請確保通過完整的測試套件檢查：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

---

## 📖 延伸文件與社群交流

- [系統架構全景](ARCHITECTURE.md) — 深入了解底層架構設計與各子系統互動。
- [本地開發指南](DEVELOPMENT.md) — 快速搭建本地開發、除錯與基準測試環境。
- [專案路線圖](ROADMAP.md) — 查看各版本研發里程碑與長遠演進計畫。
- [貢獻指南](CONTRIBUTING.md) — 如何提交高品質程式碼以及新增新的語言適配器。
- [AI 智能體研發指南](docs/AI_AGENT_WORKFLOW.md) — 面向 AI Coding Agent 的開發最佳實踐。

---

## 📄 授權條款與免責聲明

Fish 遵循 [MIT 開源授權條款](LICENSE)。

> **免責聲明：** 本專案是一個完全獨立的構建編排系統。名稱中帶有 "fish" 的其他獨立專案（例如 `fish-shell`、`fish-image` 等）與本專案不存在任何歸屬、贊助或背書關係。
