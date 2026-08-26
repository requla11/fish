# 對比矩陣：Fish 與其他主流構建系統

Fish 使用 Rust 2024 打造，專為現代多語言 Monorepo 設計。以下是 Fish 與 Bazel、Turborepo 以及 Buck2 的客觀橫向對比：

| 功能 / 維度 | Fish | Turborepo | Bazel | Buck2 |
| :--- | :--- | :--- | :--- | :--- |
| **核心實現語言** | Rust 2024 | Go / Rust | Java / C++ | Rust |
| **多語言支援** | 原生多語言（11+ 工具鏈） | 主打 JS / TS | 多語言 (Starlark 規則) | 多語言 (Starlark 規則) |
| **設定模型** | 統一 `fish.toml` / 自動發現 | `turbo.json` | `WORKSPACE` + `BUILD.bazel` | `BUCK` 文件 |
| **上手與設定難度** | 較低（零設定自動識別） | 較低 | 較高（需精細宣告規則） | 較高（需精細宣告規則） |
| **雜湊引擎** | Blake3（平行樹狀雜湊） | SHA-256 | SHA-256 | Blake3 / SHA-256 |
| **CAS 壓縮與快取** | Zstandard (Zstd) + CoW | Tar.gz / Gzip | Zstd / 自訂 | Zstd / 自訂 |
| **產物落地機制** | Reflink / 寫時複製（支援回退） | 普通檔案複製 | 軟連結 / 硬連結 | Reflink / CoW |
| **內容分塊引擎** | FastCDC（16KB - 256KB 塊去重） | 完整封存檔案 | 完整封存檔案 | 分塊 CAS |
| **VFS 髒檔案解析** | 記憶體快照樹 | 磁碟遍歷 | Inotify / Watchman 守護行程 | Watchman / EdenFS |
| **語意化快取失效** | AST 介面簽名雜湊 (ABI) | 僅檔案雜湊 | 僅標頭檔編譯 | Header / rmeta 編譯 |
| **AI 智慧診斷** | 原生 IPC + 故障根因分析 | 無 | 無 | 無 |
| **可視化監控儀表板** | 原生內建 Web GUI 與 TUI | Vercel 網頁儀表板 | 第三方工具整合 | 開源命令列終端 |

---

## 架構特性詳細對比

### Fish 與 Turborepo
* **多語言支援維度:** Turborepo 主要為 JS/TS 生態打造。Fish 原生直接從專案原生清單（`Cargo.toml`, `go.mod`, `CMakeLists.txt` 等）自動發現並協同調度 11+ 種語言工具鏈。
* **儲存與傳輸效率:** Turborepo 使用標準封存包。Fish 結合 Reflink / CoW 與 FastCDC 內容分塊技術，有效降低重複磁碟 I/O 與傳輸開銷。

### Fish 與 Bazel
* **設計定位與權衡:** Bazel 專為超大規模代碼庫設計，提供極其嚴格的檔案級密封沙箱，但需要為每個目錄編寫詳盡的 `BUILD.bazel`。Fish 定位為輕量級零設定多語言任務調度器，優先保證開箱即用與開發體驗。
* **執行開銷:** Bazel 依賴常駐 JVM 守護行程。Fish 為純 Rust 原生靜態二進位檔，啟動迅速且資源佔用低。

### Fish 與 Buck2
* **工程複雜度:** Buck2 依賴 Starlark 規則與外部檔案監控服務。Fish 內建記憶體 VFS 與 GNU Jobserver 權杖池，無需複雜的構建工程維護成本即可直接投入使用。
