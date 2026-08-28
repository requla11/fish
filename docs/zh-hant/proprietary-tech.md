# Fish 獨家專有技術與下一代演算法

> 🌐 **語言導航 / 语言导航:**
> [English](../../PROPRIETARY_TECH.md) | [Tiếng Việt](../vi/proprietary-tech.md) | [简体中文](../zh-Hans/proprietary-tech.md) | [繁體中文](proprietary-tech.md) | [日本語](../ja/proprietary-tech.md)

---

## ⚡ 概述: Fish Quantum Polyglot Core (QPC)

Fish 正在開創性地研發四項專有核心演算法，旨在從根本上解決多語言程式碼倉庫（Monorepo）與分散式構建系統中的失效擴散與效能瓶頸。

```
+-------------------------------------------------------------------------------+
|                      FISH QUANTUM POLYGLOT CORE (QPC)                         |
+-------------------------------------------------------------------------------+
|  1. Poly-ABI Semantic HyperGraph (PASH)                                       |
|     --> 跨語言公共介面邊界提取與失效級聯阻斷                                  |
|                                                                               |
|  2. Iso-Semantic Morphic Fingerprinting (IS-MFP)                              |
|     --> 消除跨環境「快取懸崖（Cache Cliff）」的雙鍵 CAS 引擎                   |
|                                                                               |
|  3. Speculative Wavelet Pre-Execution (SWPE)                                  |
|     --> 基於即時 LSP 權杖流的零開銷前瞻性微編譯                               |
|                                                                               |
|  4. CAS-VLink (Virtual Jump-Table Splicer)                                    |
|     --> 繞過系統連結器的零拷貝虛擬跳轉表二進位拼接引擎                        |
+-------------------------------------------------------------------------------+
```

---

## 🔬 1. Poly-ABI Semantic HyperGraph (PASH)
* **狀態**: 積極開發中 (`crates/fish-graph`, `crates/fish-core`)
* **問題**: 現有構建系統在原始碼變更時，即使公共介面（ABI）未改變，也會使所有下游多語言目標失效。
* **機制**: 自動提取全部 11 種語言後端的公共介面邊界（PIB），計算不變的 `Symbolic Boundary Hash (SBH)`。當僅有內部邏輯變更時，PASH 會切斷跨語言失效傳播。

---

## 🧬 2. Iso-Semantic Morphic Fingerprinting (IS-MFP)
* **狀態**: 積極開發中 (`crates/fish-cache`, `crates/fish-cas`)
* **問題**: 路徑差異與環境熵導致本機與 CI 流水線之間的快取命中率降至 0%（快取懸崖）。
* **機制**: 引入雙鍵雜湊架構（`ExactKey` + `MorphicKey`），正規化 AST 結構熵並剔除路徑/時間戳記干擾，實現跨環境 >95% 的快取複用率。

---

## 🌊 3. Speculative Wavelet Pre-Execution (SWPE)
* **狀態**: 積極開發中 (`crates/fish-scheduler`, `crates/fish-incremental`)
* **問題**: 被動式構建系統必須等待儲存快速鍵或終端命令，導致開發者頻繁等待。
* **机制**: 直連 `Fish LSP Bridge` 接收擊鍵語法小波流，調度後台空閒 CPU 權杖預熱型別推導與中間程式碼上下文。

---

## ⚡ 4. CAS-VLink (Virtual Jump-Table Splicer)
* **狀態**: 積極開發中 (`crates/fish-executor`, `crates/fish-cas`)
* **問題**: 系統連結器（`ld`, `lld`）消耗大型二進位檔案 40-60% 的構建耗時。
* **機制**: 在輸出二進位中構建虛擬分發跳轉表（VBDT），零拷貝直接拼接變更段，使反覆運算連結速度提升 10 到 50 倍。
