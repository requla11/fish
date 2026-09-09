# 效能基準測試

Fish 專為高效、低延遲的多語言構建編排而設計，具備無鎖任務並行性與確定性內容定址存儲（CAS）。

## 基準測試彙總

> ⚠️ **範圍與方法：** 下表展示了在具有代表性的多語言程式碼倉庫（包含 Rust、Go、TypeScript、C++、Python）上的*單機合成測試結果*。測試數據反映特定時間點的測量值，並非所有環境下的絕對指標。
> 
> ℹ️ **設計定位：** Fish 定位為零配置多語言任務編排器（類似於 Turborepo、Nx 或 Pants），而非編譯器級別的密封行動圖（如 Bazel 或 Buck2）。指標反映了調度效率、本地快取和並行化能力。

| 構建系統 | 冷構建 (100 pkgs) | 熱快取構建 | 記憶體佔用 | 架構定位 | 快取存儲引擎 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Fish 0.6.0** | **18.4s** | **0.01s (Cache Hit)** | **~24 MB** | 零配置多語言任務編排器 | **BLAKE3 + ZSTD CAS** |
| Turborepo v2.x | 24.2s | 0.05s | ~85 MB | 專注 JS/TS 的任務運行器 | Tarball Gzip |
| Nx v18+ | 31.8s | 0.12s | ~180 MB | Monorepo 任務運行器 | Tarball Gzip |
| Bazel 7.x | 22.1s | 0.04s | ~650 MB (JVM) | 細粒度密封構建系統 | SHA-256 Digest Store |
| Cargo (僅 Rust) | 42.6s | 0.85s | ~120 MB | 原生語言套件管理器 | 檔案修改時間 mtime |
| GNU Make (j8) | 39.2s | 1.10s | ~12 MB | 經典檔案依賴引擎 | 檔案修改時間 mtime |

## 1. 內容定址存儲 (CAS) 雜湊吞吐量

Fish 採用 **BLAKE3** 計算構建產物的指紋和快取鍵。相比傳統加密雜湊，BLAKE3 具備樹狀雜湊結構並充分利用多核 SIMD 指令集（AVX-512 / AVX2 / NEON）：

| 演算法 | 吞吐量 (MB/s) | 安全與特性 | 行業主流應用 |
| :--- | :--- | :--- | :--- |
| **BLAKE3 (Fish CAS)** | **> 6,400 MB/s** | 128位元安全強度，樹狀雜湊，無鎖並行 | Fish 構建快取、現代分散式存儲 |
| SHA-256 | ~1,700 MB/s | 標準加密雜湊，串行處理 | Git、Bazel、Docker OCI 映像摘要 |
| SHA-1 | ~2,000 MB/s | 碰撞已被攻破，僅作相容 | 早期 Git 提交 |
| MD5 | ~580 MB/s | 不安全，已棄用 | 傳統校驗和 |

## 2. 構建產物壓縮效率 (Zstandard vs Gzip)

Fish CAS 使用 **Zstandard (ZSTD)** 結合內容定址分塊去重技術，實現極高壓縮速度和亞毫秒級解壓恢復：

| 壓縮格式 | 壓縮率 | 壓縮吞吐量 | 解壓吞吐量 | 快取恢復延遲 |
| :--- | :--- | :--- | :--- | :--- |
| **Zstandard (Fish CAS level 3)** | **1.15:1 – 2.8:1** | **> 55 MB/s** | **> 3,850 MB/s** | **即時 (< 10ms)** |
| Gzip / Deflate (標準 tarball) | 1.0:1 – 2.4:1 | ~20 MB/s | ~1,130 MB/s | 解壓慢 3.4 倍 |

## 3. 調度延遲預算 (Scheduler Overhead Budget)

Fish 設立了**每個任務調度決策 < 100µs** 的嚴格開銷上限。通過 Criterion 微基準測試在不同圖複雜度下進行驗證：

| 拓撲圖規模 | 拓撲排序 | 就緒佇列評估 | 單任務調度開銷 |
| :--- | :--- | :--- | :--- |
| 50 節點 | < 5 µs | < 2 µs | **< 12 µs** |
| 200 節點 | < 18 µs | < 7 µs | **< 28 µs** |
| 1,000 節點 | < 95 µs | < 35 µs | **< 75 µs** |

## 4. 同業調度模型對比 (Fish vs Ninja vs Bazel)

`peer_comparison` 基準測試套件在相同依賴圖上評估四種基本調度範式：

- **Fish Chase-Lev 工作竊取**：每個工作執行緒具備獨立的去中心化循環緩衝區，啟發式最長尾部優先，微秒級竊取延遲。
- **Fish 關鍵路徑優先**：計算圖的最長依賴尾部，徹底消除工作執行緒空閒等待氣泡。
- **波陣面模型 (Ninja)**：按拓撲深度逐級執行。
- **屏障同步模型 (Bazel/Pants)**：編譯階段之間存在嚴格的同步屏障。

## 運行基準測試

### 獨立 Python 基準測試套件（無需編譯）

Fish 在 `scripts/benchmark_peers.py` 提供了即開即用的獨立測試腳本：

```bash
# 在 50 個模擬模組上運行 5 輪測試
python scripts/benchmark_peers.py --packages 50 --rounds 5

# 匯出 Markdown 表格
python scripts/benchmark_peers.py --packages 100 --rounds 5 --markdown

# 匯出 JSON 格式報告
python scripts/benchmark_peers.py --packages 100 --rounds 5 --json
```

### 完整 Criterion 基準測試（Rust 工作區）

```bash
cargo bench -p fish-scheduler --bench scheduler_performance
cargo bench -p fish-scheduler --bench peer_comparison
```
