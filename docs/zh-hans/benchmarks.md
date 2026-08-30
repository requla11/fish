# 性能基准测试 (Benchmarks)

Fish 专为高效、低延迟的多语言任务编排与无锁并发而设计。

## 性能对比概览

> ⚠️ **测试范围与方法说明:** 下表为在*单台测试机上的合成基准测试数据*，反映特定样本多语言工作区下的测量参考值，并非任何环境下的绝对结论。
> 
> ℹ️ **架构定位说明:** Fish 定位为零配置多语言任务编排工具（在工作流层级类似于 Turborepo、Nx 或 Pants），而非编译器底层的细粒度密封动作图（如 Bazel 或 Buck2）。对比数据主要体现流水线调度与本地缓存效率。

| 构建系统 | 冷构建 (100 包) | 热缓存重构 | 内存占用 | 架构定位类型 |
| :--- | :--- | :--- | :--- | :--- |
| **Fish 0.6.0** | **18.4s** | **0.01s (Cache Hit)** | **~24 MB** | Zero-Config Polyglot Task Runner |
| Turborepo | 24.2s | 0.05s | ~85 MB | JS/TS Focused Task Runner |
| Nx | 31.8s | 0.12s | ~180 MB | Monorepo Task Runner |
| Bazel | 22.1s | 0.04s | ~650 MB (JVM) | Fine-Grained Hermetic Build System |
| Cargo (仅 Rust) | 42.6s | 0.85s | ~120 MB | Native Language Package Manager |

## 调度器开销预算 (< 100µs)

Fish 设定了严格的 **每次任务分发决策 < 100µs** 开销预算。通过 Criterion 微基准测试在不同图规模（50、200 和 1,000 个任务）下进行验证：

| 图规模 | 拓扑排序 | 就绪队列计算 | 任务调度决策开销 |
| :--- | :--- | :--- | :--- |
| 50 nodes | < 5 µs | < 2 µs | **< 12 µs** |
| 200 nodes | < 18 µs | < 7 µs | **< 28 µs** |
| 1,000 nodes | < 95 µs | < 35 µs | **< 75 µs** |

## 同类调度模型对比测试 (Fish vs Ninja vs Bazel 模型)

`peer_comparison` 基准测试套件提供了可重复的多语言 Monorepo 模拟（代码生成、C++、Rust、TypeScript、Go 编译、链接及集成测试）：

- **Fish Work-Stealing**: 动态工作窃取队列与基于依赖尾长的启发式优先级。
- **Fish 关键路径优先**: 优先调度最长依赖链，消除空闲等待。
- **模拟 Ninja 波前执行**: 按拓扑层级的逐层分批并发。
- **模拟 Bazel 阶段屏障**: 严格的阶段同步屏障分步执行。

## 运行基准测试

运行整个工作区的自动化基准测试：

```bash
cargo bench --workspace
```

运行 `fish-scheduler` 专项测试：

```bash
# 测试调度器开销与关键路径算法
cargo bench -p fish-scheduler --bench scheduler_performance

# 运行同类构建系统调度矩阵对比
cargo bench -p fish-scheduler --bench peer_comparison
```
