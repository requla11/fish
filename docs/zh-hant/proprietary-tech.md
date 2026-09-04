# Fish 進階演算法與架構創新

> 🌐 **多語言導覽 / Language Navigation:**
> [English](../proprietary-tech.md) | [Tiếng Việt](../vi/proprietary-tech.md) | [日本語](../ja/proprietary-tech.md) | [简体中文](../zh-hans/proprietary-tech.md) | [繁體中文](proprietary-tech.md)

---

## ⚡ 概述: Fish 核心演算法創新

Fish 整合了四種專用演算法，旨在解決多語言 Monorepo 中的擴展性、快取失效與增量延遲挑戰：

```
+-------------------------------------------------------------------------------+
|                            FISH 核心演算法創新                                |
+-------------------------------------------------------------------------------+
|  1. Poly-ABI Semantic HyperGraph (PASH)                                       |
|     --> 介面邊界符號提取與下游失效級聯截斷                                    |
|                                                                               |
|  2. Iso-Semantic Morphic Fingerprinting (IS-MFP)                              |
|     --> 雙鍵 CAS 規範化，消除跨環境快取未命中                                |
|                                                                               |
|  3. Speculative Wavelet Pre-Execution (SWPE)                                  |
|     --> 編輯事件能量分級與主動相依預熱                                        |
|                                                                               |
|  4. Virtual Binary Dispatch Table (CAS-VLink)                                 |
|     --> 用於快速增量迭代的記憶體符號調度覆蓋層                                |
+-------------------------------------------------------------------------------+
```

---

## 🔬 1. Poly-ABI Semantic HyperGraph (PASH)
* **位置**: `crates/fish-graph`, `crates/fish-core`
* **問題**: 傳統建置系統在任何上游原始檔發生變更時，都會使所有下游目標失效，即使公開介面（API/簽章）完全未變。
* **機制**:
  * 掃描全部 11 個支援的後端導出的公開介面簽章。
  * 計算確定性的符號邊界雜湊 `Symbolic Boundary Hash (SBH)`。
  * 當內部實作變更而 `SBH` 保持相同時，PASH 截斷失效級聯，避免冗餘重建。

---

## 🧬 2. Iso-Semantic Morphic Fingerprinting (IS-MFP)
* **位置**: `crates/fish-cache`, `crates/fish-cas`
* **問題**: 工作區路徑差異、格式差異與環境波動常導致本地開發機與 CI Runner 之間的快取命中率降為 0%。
* **機制**:
  * 實作包含 `ExactKey` 與 `MorphicKey` 的**雙鍵雜湊架構**。
  * 規範化相對路徑（將 Windows 反斜線轉換為斜線）並過濾易變環境變數。
  * 未精確命中時回退至同態匹配，最大化不同環境間的快取復用率。

---

## 🌊 3. Speculative Wavelet Pre-Execution (SWPE)
* **位置**: `crates/fish-incremental`
* **問題**: 被動式建置系統需等待手動儲存或指令執行，操作累積導致開發者等待延遲。
* **機制**:
  * 將編輯器事件差異劃分為不同能量級別（`TrivialWhitespace`、`CommentOnly`、`InternalStatement`、`GlobalInterface`）。
  * 在完整建置執行前，於背景記憶體中預先準備任務相依狀態。

---

## ⚡ 4. Virtual Binary Dispatch Table (CAS-VLink)
* **位置**: `crates/fish-executor`
* **問題**: 在快速增量迭代中，頻繁進行完整的二進位重新連結會帶來明顯開銷。
* **机制**:
  * 在記憶體中維護映射符號地址與位元組碼塊的 `VirtualBinaryDispatchTable`。
  * 產生結構化執行時期二進位覆蓋層（`VLINK_DISPATCH_HEADER_V1`），支援快速符號替換與測試。
