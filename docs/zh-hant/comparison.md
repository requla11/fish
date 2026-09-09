# 對比矩陣：Fish 與其他構建系統

Fish 是專為現代多語言單體程式碼倉庫（Polyglot Monorepos）打造的構建編排系統，使用 Rust 2024 開發。以下是與 Bazel、Turborepo 和 Buck2 的客觀技術對比：

| 功能 / 維度 | Fish | Turborepo | Bazel | Buck2 |
| :--- | :--- | :--- | :--- | :--- |
| **開發語言** | Rust 2024 | Go / Rust | Java / C++ | Rust |
| **多語言支援** | 多語言原生支援 (11+ 工具鏈) | 專注 JS / TS | 多語言 (Starlark 規則) | 多語言 (Starlark 規則) |
| **配置模型** | `fish.toml` / 零配置自動發現 | `turbo.json` | `WORKSPACE` + `BUILD.bazel` | `BUCK` 規則檔案 |
| **配置複雜度** | 極低 / 零配置 | 低 | 高 (需精細宣告每個目標) | 高 (需精細宣告每個目標) |
| **指紋雜湊演算法** | Blake3 (多核並行樹狀雜湊) | SHA-256 | SHA-256 | Blake3 / SHA-256 |
| **CAS 產物壓縮** | Zstandard (Zstd) + CoW | Tar.gz / Gzip | Zstd / 自訂 | Zstd / 自訂 |
| **產物還原機制** | Reflink / CoW (寫入時複製，降級拷貝) | 檔案全量拷貝 | 符號連結 / 硬連結 | Reflink / CoW |
| **資料分塊去重** | FastCDC (16KB - 256KB 內容分塊) | 整包封存檔案 | 整包封存檔案 | 分塊 CAS |
| **虛擬檔案系統 (VFS)**| 記憶體快照樹 (In-Memory Tree) | 磁碟檔案掃描 | Inotify / Watchman 守護程序 | Watchman / EdenFS |
| **語意級失效檢測** | AST 介面雜湊 (ABI 級別) | 純檔案內容雜湊 | 標頭檔級編譯 (Header-only) | 標頭檔 / rmeta 編譯 |
| **智慧診斷 (AI)** | 原生 IPC + 啟發式錯誤修復解釋 | 無 | 無 | 無 |
| **互動式看板** | 內建 Web GUI + 終端 TUI | Vercel 網頁應用 | 第三方控制台 | 開源終端控制台 |

---

## 詳細架構剖析

### Fish 對比 Turborepo
* **語言適用範圍：** Turborepo 主要面向 JavaScript/TypeScript 生態。Fish 原生掃描並編排 11 種以上原生工具鏈（Cargo、Go modules、CMake、Python、Docker 等），直接解析各語言標準配置。
* **儲存與 I/O 效率：** Turborepo 採用標準 tarball 壓縮。Fish 採用檔案系統寫入時複製（Reflink CoW）與 FastCDC 去重分塊，最大限度降低磁碟 I/O 和網路傳輸。

### Fish 對比 Bazel
* **設計理念與權衡：** Bazel 專為需要極嚴格密封沙箱（Hermetic Sandbox）的超大型程式碼庫設計，每個構建目標必須宣告 `BUILD.bazel`。Fish 定位為輕量級零配置任務編排器，優先考慮極速開箱體驗與極低資源消耗（Fish 記憶體佔用約 24 MB，而 Bazel JVM 守護程序需 650 MB 以上）。
* **執行階段架構：** Bazel 依賴重量級 JVM 守護程序與沙箱包裝層。Fish 為單一獨立 Rust 本機二進位程式，啟動延遲小於 15ms。

### Fish 對比 Buck2
* **工作流程易用性：** Buck2 是面向大規模倉庫的高效能構建系統，採用 Starlark 規則。Fish 聚焦於開箱即用，內建記憶體 VFS 與 GNU jobserver 資源池，開發者無需編寫複雜配置。

---

## 實證案例研究：Bazel vs Fish（基於 `bazelbuild/examples`）

> ⚠️ **聲明 —— 僅供參考：**
> 本案例研究中的實測數據記錄於一台代表性 Windows x86_64 開發者工作站（4 核 CPU，約 3.8 GB RAM），測試對象為 Google 官方範例倉庫 [`bazelbuild/examples`](https://github.com/bazelbuild/examples)（提交雜湊 `3c479f4`）。
> **本數據僅作為技術對比與概念驗證參考**。實際生產環境中的構建效能將因硬體規格、磁碟 I/O 速度、網路下載頻寬（下載遠端工具鏈規則時）及快取預熱狀態而有所不同。Bazel 提供編譯器級別的密封隔離保證，需要較高的初始啟動開銷；而 Fish 專注於提供零配置極速本機執行體驗。

### 測試環境設定

針對 `bazelbuild/examples` 中 Go 語言教學的全部三個階段（`stage1`, `stage2`, `stage3`）進行全面對比測試：
- **快取完全清理流程：**
  - **Bazel：** 執行 `bazel clean --expunge` 完全清除輸出快取、沙箱並終止背景 JVM 處理程序。
  - **Fish：** 徹底刪除 `.fish/cache` 目錄及本機生成目錄 `build/`。
- **構建目標範圍：** 純二進位編譯產物生成（Bazel 對應 `go_binary`，Fish 對應停用測試的 `go build`）。

### 實測數據對比表

| 測試模組 | 構建目標 | Bazel 7.4.0 (冷構建) | Bazel 7.4.0 (熱快取命中) | Fish 0.6.0 (冷構建) | Fish 0.6.0 (熱快取命中) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Go Tutorial Stage 1** | `hello` | 165.53s | 23.55s | **1.08s** | **0.00092s (0.9ms)** |
| **Go Tutorial Stage 2** | `print_fortune` | 145.89s | 23.40s | **1.69s** | **0.00095s (0.9ms)** |
| **Go Tutorial Stage 3** | `fortune_test` | 149.68s | 23.70s | **0.99s** | **0.00088s (0.8ms)** |
| **3 個模組合併總計** | **全部 3 個目標** | **461.10s (~7.7 分鐘)** | **~70.65s** | **3.76s** | **0.00275s (2.7ms)** |

### 技術成因深度分析

1. **冷構建時間差異分析 (461.10s vs 3.76s)：**
   - **Bazel：** 必須冷啟動 Java 虛擬機器（JVM），下載 Bazel 7.4 安裝套件，拉取 `rules_go`，分析 101 個套件並配置超過 10,800 個構建目標，在沙箱中編譯 `builder.exe` 輔助工具及 Go 標準庫。
   - **Fish：** 直接複用本機系統安裝的 Go 工具鏈，初始化時間小於 15ms，省去龐大的沙箱環境下載，直接將任務送入去中心化工作竊取佇列。

2. **熱快取命中時間差異分析 (~70.65s vs 0.00275s)：**
   - **Bazel：** 即使程式碼毫無變動，Bazel 仍需連接 JVM 守護程序，重新求值 Starlark 指令碼並比對數千個目標的雜湊值。
   - **Fish：** 利用 BLAKE3 樹狀雜湊在微秒級內比對檔案元數據與內容指紋。確認無修改後，**100% 命中 CAS 快取**，3 個專案的總檢查時間在 3 毫秒以內完成。
