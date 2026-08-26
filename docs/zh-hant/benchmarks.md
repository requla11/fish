# 效能基準測試 (Benchmarks)

Fish 專為超低延遲建置調度與無鎖高並發而設計。

## 效能對比概覽

| 建置系統 | 冷建置 (100 包) | 熱快取重構 | 記憶體佔用 | 多語言支援 |
| :--- | :--- | :--- | :--- | :--- |
| **Fish 0.6.0** | **18.4s** | **0.01s (Cache Hit)** | **~24 MB** | **原生支援 11+ 語言** |
| Turborepo | 24.2s | 0.05s | ~85 MB | 專注於 JS/TS |
| Nx | 31.8s | 0.12s | ~180 MB | JS/TS Monorepo |
| Bazel | 22.1s | 0.04s | ~650 MB (JVM) | 多語言支援 |
| Cargo (僅 Rust) | 42.6s | 0.85s | ~120 MB | 僅支援 Rust |

## 調度器開銷預算 (< 100µs)

Fish 設定了嚴格的 **每次任務分派決策 < 100µs** 開銷預算。透過 Criterion 微基準測試在不同圖規模（50、200 和 1,000 個任務）下進行驗證：

| 圖規模 | 拓撲排序 | 就緒佇列計算 | 任務調度決策開銷 |
| :--- | :--- | :--- | :--- |
| 50 nodes | < 5 µs | < 2 µs | **< 12 µs** |
| 200 nodes | < 18 µs | < 7 µs | **< 28 µs** |
| 1,000 nodes | < 95 µs | < 35 µs | **< 75 µs** |

## 同類調度模型對比測試 (Fish vs Ninja vs Bazel)

`peer_comparison` 基準測試套件提供了可重複的多語言 Monorepo 模擬（代碼生成、C++、Rust、TypeScript、Go 編譯、連結及整合測試）：

- **Fish Work-Stealing**: 動態工作竊取佇列與基於相依尾長的啟發式優先順序。
- **Fish 關鍵路徑優先**: 優先調度最長相依鏈，消除閒置等待。
- **模擬 Ninja 波前執行**: 按拓撲層級的逐層分批並行。
- **模擬 Bazel 階段屏障**: 嚴格的階段同步屏障分步執行。

## 執行基準測試

執行整個工作區的自動化基準測試：

```bash
cargo bench --workspace
```

執行 `fish-scheduler` 專項測試：

```bash
# 測試調度器開銷與關鍵路徑演算法
cargo bench -p fish-scheduler --bench scheduler_performance

# 執行同類建置系統調度矩陣對比
cargo bench -p fish-scheduler --bench peer_comparison
```
