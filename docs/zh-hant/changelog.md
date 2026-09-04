# 更新日誌與版本歷史

Fish 專案的所有重要變更均記錄於此。

格式基於 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)，並遵循 [語意化版本規範](https://semver.org/spec/v2.0.0.html)。

## [v0.6.0] - 2026-08-25

### 新增
- **跨語言 Protobuf 協定**：在 Rust、Go 和 Python 之間實現二進位 Google Protocol Buffers wire 編解碼，無需笨重的外部編譯器依賴。
- **Wasm 外掛程式引擎與安全稽核**：沙箱化 WebAssembly 外掛程式支援、權限能力稽核（`fish plugin audit`）及 Ed25519 密碼學簽章驗證。
- **ZSTD 內容定址存儲 (CAS)**：極速 BLAKE3 樹狀雜湊與多執行緒 Zstandard 壓縮，構建確定性 L1/L2 快取。
- **11 種多語言生態後端**：針對 Rust、Go、TypeScript/Node、Python、C/C++、Docker、Java、.NET、Swift、Dart 和 Zig 的原生零配置支援。
- **自適應並行與工作竊取**：基於 Chase-Lev 雙端佇列去中心化工作竊取調度，結合關鍵路徑啟發式優化與 RAM 記憶體防顛簸控制。

### 優化
- 依賴環檢測重構，提供完整閉環路徑診斷而非籠統報錯。
- 本地快取命中時自動解壓還原任務聲明的產物檔案至磁碟。

## [v0.5.0] - 2026-08-24

### 新增
- **5 語言文件門戶**：基於 VitePress 的完整文件系統，支援英語、越南語、簡體中文、繁體中文和日語。
- **分散式協調器 (Go)**：高吞吐工作節點協調器，具備心跳追蹤與 HTTP/Protobuf 任務分派能力。
- **AI 錯誤分析與修復 (Python)**：基於子處理程序通道的編譯器錯誤診斷解析與預測性預熱。

## [v0.3.0] - 2026-08-21

### 新增
- **IDE 擴充功能**：官方 VS Code 擴充功能與 Language Server Protocol (`fish lsp`) 語言伺服器整合。
- **互動式 TUI 控制台**：即時多執行緒構建進度、CPU/RAM 佔用率與瀑布流視覺化。
- **eBPF 動態追蹤**：在 Linux 核心層捕獲檔案存取與動態依賴。

## [v0.2.0] - 2026-08-10

### 新增
- **三引擎核心架構**：Rust 2024 核心引擎配合 Go 分散式網路與 Python AI 優化服務。
- **指紋快取引擎**：基於 BLAKE3 的超高吞吐任務指紋與增量變更檢測。
- **GNU Jobserver 資源池**：全域並發度治理，防止編譯資源耗盡。

## [v0.1.0] - 2026-08-01

### 新增
- Fish 首個實驗性版本發布，提供 Rust 與 TypeScript 構建支援。
