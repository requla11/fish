# Fish 高级算法与架构创新

> 🌐 **多语言导航 / Language Navigation:**
> [English](../proprietary-tech.md) | [Tiếng Việt](../vi/proprietary-tech.md) | [日本語](../ja/proprietary-tech.md) | [简体中文](proprietary-tech.md) | [繁體中文](../zh-hant/proprietary-tech.md)

---

## ⚡ 概述: Fish 核心算法创新

Fish 整合了四种专用算法，旨在解决多语言 Monorepo 中的扩展性、缓存失效与增量延迟挑战：

```
+-------------------------------------------------------------------------------+
|                            FISH 核心算法创新                                  |
+-------------------------------------------------------------------------------+
|  1. Poly-ABI Semantic HyperGraph (PASH)                                       |
|     --> 接口边界符号提取与下游失效级联截断                                    |
|                                                                               |
|  2. Iso-Semantic Morphic Fingerprinting (IS-MFP)                              |
|     --> 双键 CAS 规范化，消除跨环境缓存未命中                                |
|                                                                               |
|  3. Speculative Wavelet Pre-Execution (SWPE)                                  |
|     --> 编辑事件能量分级与主动依赖预热                                        |
|                                                                               |
|  4. Virtual Binary Dispatch Table (CAS-VLink)                                 |
|     --> 用于快速增量迭代的内存符号调度覆盖层                                  |
+-------------------------------------------------------------------------------+
```

---

## 🔬 1. Poly-ABI Semantic HyperGraph (PASH)
* **位置**: `crates/fish-graph`, `crates/fish-core`
* **问题**: 传统构建系统在任何上游源文件发生变更时，都会使所有下游目标失效，即使公共接口（API/签名）完全未变。
* **机制**:
  * 扫描全部 11 个支持的后端导出的公共接口签名。
  * 计算确定性的符号边界哈希 `Symbolic Boundary Hash (SBH)`。
  * 当内部实现变更而 `SBH` 保持相同时，PASH 截断失效级联，避免冗余重建。

---

## 🧬 2. Iso-Semantic Morphic Fingerprinting (IS-MFP)
* **位置**: `crates/fish-cache`, `crates/fish-cas`
* **问题**: 工作区路径差异、格式差异和环境波动常导致本地开发机与 CI Runner 之间的缓存命中率降为 0%。
* **机制**:
  * 实现包含 `ExactKey` 与 `MorphicKey` 的**双键哈希架构**。
  * 规范化相对路径（将 Windows 反斜杠转换为斜杠）并过滤易变环境变量。
  * 未精确命中时回退至同态匹配，最大化不同环境间的缓存复用率。

---

## 🌊 3. Speculative Wavelet Pre-Execution (SWPE)
* **位置**: `crates/fish-incremental`
* **问题**: 被动式构建系统需等待手动保存或命令执行，操作累积导致开发者等待延迟。
* **机制**:
  * 将编辑器事件差异划分为不同能量级别（`TrivialWhitespace`、`CommentOnly`、`InternalStatement`、`GlobalInterface`）。
  * 在完全构建执行前，于后台内存中预先准备任务依赖状态。

---

## ⚡ 4. Virtual Binary Dispatch Table (CAS-VLink)
* **位置**: `crates/fish-executor`
* **问题**: 在快速增量迭代中，频繁进行完整的二进制重新链接会带来明显开销。
* **机制**:
  * 在内存中维护映射符号地址与字节码块的 `VirtualBinaryDispatchTable`。
  * 生成结构化运行时二进制覆盖层（`VLINK_DISPATCH_HEADER_V1`），支持快速符号替换与测试。
