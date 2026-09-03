# Fish 專案發展藍圖 (Roadmap)

> 🌐 **多語言與貢獻:** 想要將此文件翻譯或改進為您使用的語言？請參閱 [翻譯指南](TRANSLATION.md)。

本文件概述了 Fish 的戰略開發藍圖，涵蓋已完成的里程碑、短期與中期目標、長期願景以及前沿研究方向。

---

## 🎯 願景

Fish 旨在成為多語言 Monorepo 與分散式開發環境中最有效率、最穩健且對開發者最友善的建置編排系統，以單一語言的 **Rust 核心（28 個 crate，Rust 2024，MSRV 1.88+）與 11 個多語言後端**為驅動。可選的 Go/Python 輔助服務及 `proto/` 契約屬於前瞻性草案（詳見 `ARCHITECTURE.md`）。

我們優先最佳化的核心目標：

1. **實際建置時間 (Wall-clock time)** — 終端使用者唯一能直接感知的指標。
2. **快取效率** — 命中率，跨機器與跨地域的建置產物復用。
3. **可信度** — 快取的每個位元組都與輸入確定性相符。
4. **工具輸出的真實性** — 杜絕虛假診斷與偽造的建置成功。

---

## 🚀 當前里程碑 (v0.2.x) — 已完成

### 第 1 階段: 核心引擎與多語言基礎
- [x] **Rust 核心架構**: 單一語言 Rust 工作區（28 個 crate，resolver = "2"，MSRV 1.88+）- 無 `prost`/`tonic` 相依；分散式功能直接使用 HTTP/JSON。
- [x] **11 個語言後端**: Rust、Go、TypeScript/Node.js、Python、C/C++、Docker、Java、.NET、Swift、Dart、Zig。
- [x] **Protobuf 契約草案**: `proto/fish/v1/build.proto`、`ai.proto` 與 `coordinator.proto` 設計草案。
- [x] **Blake3 CAS 與兩階段清理**: 帶有 Zstandard 壓縮的高吞吐內容定址儲存。
- [x] **GNU Jobserver 執行緒池**: 跨編譯器的全域執行緒權杖分配與動態調度。
- [x] **CI/CD 產生器**: 自動產生 GitHub Actions、GitLab CI、CircleCI、Bitbucket 設定。
- [x] **5 種語言文件**: 已在 GitHub Pages 上線（英文、中文簡體、中文繁體、日文、越南文）。

---

## ⚡ 短期目標 (v0.3.x) — 已完成: 開發者體驗與協定

### 1. IDE 與編輯器整合
- [x] **VS Code 擴充套件**: 互動式 DAG 相依圖檢視器、一鍵任務執行與內嵌診斷。
- [x] **JetBrains 外掛套件**: 為 CLion、IntelliJ IDEA 與 Rider 提供 ToolWindow 與 LSP 支援。
- [x] **LSP 語言伺服器橋接**: `fish.toml` 即時自動完成與工作區診斷。

### 2. 高效能 IPC 與服務橋接
- [x] **Daemon IPC 串流**: Rust CLI 與 Python 服務間次毫秒級 JSON-RPC 2.0 通訊。
- [x] **gRPC 遠端執行 API (REAPI)**: 分散式 Worker 叢集的完整 REAPI v2 用戶端。
- [x] **eBPF 檔案追蹤**: Linux 核心級輸入/輸出擷取與動態相依分析。

### 3. 智慧診斷與 CLI 增強
- [x] **互動式 AI 診斷器**: 主動診斷與自動修復建議（`fish doctor --fix`）。
- [x] **終端 UI (TUI) 增強**: 即時 CPU/RAM 迷你圖與任務瀑布流檢視。

---

## 🌟 中期目標 (v0.4.x - v0.5.x) — 重點: 雲端原生分散式基礎設施、AI 與成本智慧

### 1. 雲端原生分散式基礎設施
- [x] **Kubernetes Operator (Go)**: 用於彈性伸縮 Worker 叢集的 CRD（`go/pkg/k8s`）。
- [x] **競價執行個體 (Spot) 容錯最佳化**: 節點被搶占時的容錯任務遷移。
- [x] **跨地域快取複寫**: P2P 對等 CAS 產物地理分散式同步。

### 2. 機器學習與預測最佳化
- [x] **建置耗時預測**: 基於 EMA 與歷史遙測的耗時估算。
- [x] **不穩定測試 (Flaky Test) 自動隔離**: 統計學非確定性測試檢測與隔離。
- [x] **投機性預熱 (Speculative Pre-Warming)**: 預測變更並在背景預先編譯。

### 3. 遙測、可觀測性與團隊協作
- [x] **OpenTelemetry 整合**: 貫穿所有建置步驟的 OTLP 分散式追蹤。
- [x] **Web 分析儀表板**: 內部 HTTP 伺服器提供建置提速與 Flamegraph。
- [x] **雲端成本計算機**: AWS/GCP/Azure 即時成本估算與節約報告（`fish cost-estimate`）。
- [x] **分散式追蹤聚合**: 合併來自所有 Worker 的 Span。
- [x] **建置效能衰退警報**: 自動檢測 PR 與基準分支間的耗時退化。

---

## 🧭 v0.6.x — 重點: 可靠性、氣密性與供應鏈安全

- [x] **氣密性工具鏈下載器**: 帶有 SHA-256 校驗的工具鏈自動拉取。
- [x] **工具鏈鎖定檔案**: `fish.lock` 版本確定性鎖定。
- [x] **離線模式保障**: `--offline` 旗標保證完全離線確定性。
- [x] **追蹤重放 (Trace Replay)**: 記錄並重放建置執行以驗證氣密性。
- [x] **逐位元組重現性認證**: 產物目錄 BLAKE3 雜湊比對。
- [x] **環境漂移檢測器**: 警告編譯器及作業系統環境波動。
- [x] **沙盒原則設定檔**: `strict`/`default`/`trusted` 權限分級。
- [x] **產物簽章驗證門禁**: 拒絕未經驗證的遠端 CAS 產物。
- [x] **相依漏洞稽核整合**: 接入 RustSec/OSV 漏洞源（`fish-security/src/osv.rs`）。

---

## 🤖 v0.7.x — 重點: 原生 AI 建置體驗

- [x] **編譯器對齊的自動修復**: `fish fix` 依據編譯器輸出套用安全修補程式。
- [x] **自然語言建置查詢**: `fish why --ask` 基於真實數據解答建置原因。
- [x] **自適應資源調控**: 基於 P90 記憶體預測動態調整工作池容量。
- [x] **智慧測試篩選 (Test Selection)**: 依據相依影響圖略過無關測試。
- [x] **建置時序資料儲存**: 本地 SQLite WAL 記錄指標。

---

## 🏛️ 長期願景 (v1.0+) — 重點: 企業級與零信任架構

- [x] **MicroVM 硬體隔離**: 基於 Firecracker 的輕量級虛擬機器隔離執行。
- [ ] **企業身分驗證 (SSO / OIDC)**: RBAC 權限控制與稽核日誌。
- [ ] **密碼學供應鏈憑證 (SLSA Level 3)**: 產生防篡改 in-toto 證明。
- [x] **高可用協調器 (HA Coordinator)**: Go 控制面中的 Raft 一致性通訊協定。
- [x] **多租戶快取隔離**: 按團隊配額劃分 CAS 命名空間。
- [x] **多語言 AST 子樹快取**: 函式粒度的增量編譯。
- [x] **全域 P2P 網格發發**: 受 BitTorrent 啟發的產物共享。

---

## 🚀 前沿研究方向 (v2.0 Moonshots)

- [ ] **編譯器查詢掛鉤**: 與 rustc/tsc/clang 的深度增量單元整合。
- [x] **自癒建置 (Self-Healing Builds)**: 失敗時自動進行 git bisect 並準備修復 PR。
- [x] **碳感知調度 (Carbon-Aware Scheduling)**: 調度工作負載至低碳排放時段。
- [ ] **全球建置網格聯盟**: 跨組織匿名共享通用相依 CAS 區塊。
- [ ] **自然語言建置撰寫**: 從自然語言描述自動產生型別安全的 `fish.yaml`。
