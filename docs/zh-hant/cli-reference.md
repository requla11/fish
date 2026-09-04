# Fish CLI 命令列參考手冊

> 🌐 **多語言與貢獻:** 想要將此文件翻譯或改進為您使用的語言？請參閱 [翻譯指南](TRANSLATION.md)。

Fish 命令列介面的完整參考手冊，涵蓋所有可用子命令、選項旗標及設定。

---

## 🧭 基本語法與全域選項

```bash
fish [OPTIONS] <COMMAND>
```

### 全域旗標 (Global Flags)

| 旗標 (Flag) | 說明 | 預設值 |
|---|---|---|
| `--experimental` | 啟用實驗性功能。 | `false` |
| `--offline` | 停用網路存取，純離線執行並快速失敗。 | `false` |
| `-v, --verbose` | 啟用詳細診斷記錄與執行輸出。 | `false` |
| `-j, --jobs <N>` | 最大並行工作執行緒數。 | CPU 核心數 |
| `--no-cache` | 停用本機與遠端快取。 | `false` |
| `--cache-dir <PATH>` | 本機快取目錄路徑（預設: `~/.fish/cache`）。 | 系統預設 |

---

## 🛠️ 子命令完整清單

---

### `fish init`
初始化 Fish 設定檔（`fish.toml`）並掃描多語言工作區。

```bash
fish init [OPTIONS]
```
- `-p, --path <PATH>`: 初始化的目標目錄。
- `-f, --force`: 強制覆寫已存在的設定檔。
- `--describe <DESC>`: 使用自然語言描述專案結構（用於 AI 輔助設定）。

---

### `fish new`
基於內建模組建立新專案或子套件。

```bash
fish new <NAME> [OPTIONS]
```
- `-t, --template <TEMPLATE>`: 模組名稱（如: `rust`、`ts`、`go`、`polyglot`）。
- `-p, --path <PATH>`: 目標路徑。

---

### `fish build`
執行工作區各套件的建置任務。

```bash
fish build [OPTIONS] [PATH]
```
- `-j, --jobs <N>`: 限制並行任務數。
- `-v, --verbose`: 列印詳細建置步驟。
- `--no-cache`: 略過快取。
- `--sandbox`: 在安全沙盒中執行任務。
- `--apple`: 透過 `apple` 氣密沙盒執行。
- `--profile <FILE>`: 輸出 Chrome trace JSON 效能分析檔案。
- `--tui`: 啟用互動式終端機 UI。
- `--remote-cache <URL>`: 遠端快取伺服器位址（HTTP 或 gRPC REAPI）。
- `--remote-workers <URL>`: 遠端分散式 Worker 叢集。
- `--ram-limit <PCT>`: 當可用記憶體低於該百分比時自動降低並行度。
- `--semantic`: 啟用 AST 級語意快取。
- `--reflink`: 從 CAS 復原產物時使用寫入時複製 (reflink)。
- `--critical-path`: 優先調度關鍵路徑上的任務。
- `--explain`: 列印各任務被重新建置的原因。
- `--otel-endpoint <URL>`: 匯出 OpenTelemetry 追蹤至 OTLP 收集器。

---

### `fish check`
僅執行快速型別檢查與靜態分析，無需連結產生完整二進位檔案。

```bash
fish check [OPTIONS] [PATH]
```

---

### `fish test`
執行工作區中的所有測試套件。

```bash
fish test [OPTIONS] [PATH]
```
- `--quarantine-flaky`: 自動檢測並隔離不穩定測試 (flaky test)。
- `--test-threads <N>`: 測試並行執行緒數。

---

### `fish clean`
清理建置暫存檔案並釋放快取空間。

```bash
fish clean [OPTIONS]
```
- `--all`: 清空本機 CAS 與 L1/L2 快取。
- `--dry-run`: 預覽將要刪除的檔案清單而不實際刪除。

---

### `fish run`
建置並執行指定的可執行二進位目標。

```bash
fish run -p <PACKAGE> [--bin <BINARY>] [-- <ARGS>...]
```

---

### `fish graph`
匯出並視覺化工作區相依有向無環圖 (DAG)。

```bash
fish graph [OPTIONS]
```
- `--format <FORMAT>`: 匯出格式（`dot`, `json`, `mermaid`, `svg`）。
- `--output <FILE>`: 寫入圖表到指定檔案。

---

### `fish watch`
監聽檔案修改並自動觸發增量增效建置。

```bash
fish watch [OPTIONS]
```
- `--debounce <MS>`: 檔案變更去抖動緩衝時間（預設: 200ms）。

---

### `fish query`
對相依圖執行代數運算式查詢。

```bash
fish query "<EXPRESSION>"
```
- `deps(//pkg)`: 目標套件的正向相依。
- `rdeps(//pkg)`: 目標套件的反向相依。
- `allpaths(//a, //b)`: 兩個目標之間的所有路徑。
- `somepath(//a, //b)`: 兩個目標之間的最短路徑。

---

### `fish doctor`
診斷開發環境、工具鏈及 Fish 設定的健康狀態。

```bash
fish doctor [OPTIONS]
```
- `--fix`: 自動修復權限、孤立暫存檔案及 `fish.toml` 設定問題。
- `--ai`: 呼叫 AI 引擎提供深度診斷與修復方案。

---

### `fish why`
解釋某個目標套件被重新建置的具體原因。

```bash
fish why <TARGET> [OPTIONS]
```
- `--ask "<QUESTION>"`: 使用自然語言詢問重構原因。

---

### `fish fix`
依據編譯器錯誤與警告輸出套用安全的自動修復修補程式。

```bash
fish fix [OPTIONS]
```
- `--diff`: 套用前預覽 Git unified diff。
- `--apply`: 直接將修復程式碼套用至原始碼檔案。

---

### `fish affected`
比對 Git 提交或基準分支，定位受變更影響的套件清單。

```bash
fish affected --base <REF> [--head <REF>]
```

---

### `fish cache`
管理、統計與最佳化內容定址儲存 (CAS)。

```bash
fish cache <SUBCOMMAND>
```
- `prune`: 基於 LRU 與配額清理過期資料區塊。
- `stats`: 檢視快取命中率與磁碟佔用。
- `verify`: 校驗 CAS 產物的雜湊完整性。

---

### `fish cost-estimate`
估算在 AWS、GCP、Azure 上的運算資源消耗與節約金額。

```bash
fish cost-estimate [OPTIONS]
```
- `--json`: 輸出 JSON 格式供 CI/CD 流水線解析。

---

### `fish ui`
啟動即時效能分析與 DAG 圖表 Web 主控台。

```bash
fish ui [--port <PORT>] [--open]
```

---

### `fish pash`
計算並檢查路徑感知語意雜湊 (Path-Aware Semantic Hashing)。

```bash
fish pash <TARGET>
```

---

### `fish qpc`
檢查查詢管線快取 (Query Pipeline Cache) 狀態。

```bash
fish qpc <TARGET>
```

---

### `fish attest` & `fish verify`
產生與驗證建置產物的 Ed25519 密碼學簽章及 SLSA / in-toto 憑據。

```bash
fish attest --out <ATTESTATION_FILE>
fish verify --attestation <ATTESTATION_FILE>
```

---

### `fish lsp` & `fish daemon`
啟動 IDE 語言伺服器或背景常駐 IPC 守護處理程序。

```bash
fish lsp
fish daemon [--socket <PATH>]
```
