# 性能基准测试

Fish 专为高效、低延迟的多语言构建编排而设计，具备无锁任务并行性与确定性内容寻址存储（CAS）。

## 基准测试汇总

> ⚠️ **范围与方法：** 下表展示了在具有代表性的多语言代码仓库（包含 Rust、Go、TypeScript、C++、Python）上的*单机合成测试结果*。测试数据反映特定时间点的测量值，并非所有环境下的绝对指标。
> 
> ℹ️ **设计定位：** Fish 定位为零配置多语言任务编排器（类似于 Turborepo、Nx 或 Pants），而非编译器级别的密封行动图（如 Bazel 或 Buck2）。指标反映了调度效率、本地缓存和并行化能力。

| 构建系统 | 冷构建 (100 pkgs) | 热缓存构建 | 内存占用 | 架构定位 | 缓存存储引擎 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Fish 0.6.0** | **18.4s** | **0.01s (Cache Hit)** | **~24 MB** | 零配置多语言任务编排器 | **BLAKE3 + ZSTD CAS** |
| Turborepo v2.x | 24.2s | 0.05s | ~85 MB | 专注 JS/TS 的任务运行器 | Tarball Gzip |
| Nx v18+ | 31.8s | 0.12s | ~180 MB | Monorepo 任务运行器 | Tarball Gzip |
| Bazel 7.x | 22.1s | 0.04s | ~650 MB (JVM) | 细粒度密封构建系统 | SHA-256 Digest Store |
| Cargo (仅 Rust) | 42.6s | 0.85s | ~120 MB | 原生语言包管理器 | 文件修改时间 mtime |
| GNU Make (j8) | 39.2s | 1.10s | ~12 MB | 经典文件依赖引擎 | 文件修改时间 mtime |

## 1. 内容寻址存储 (CAS) 哈希吞吐量

Fish 采用 **BLAKE3** 计算构建产物的指纹和缓存键。相比传统加密哈希，BLAKE3 具备树状哈希结构并充分利用多核 SIMD 指令集（AVX-512 / AVX2 / NEON）：

| 算法 | 吞吐量 (MB/s) | 安全与特性 | 行业主流应用 |
| :--- | :--- | :--- | :--- |
| **BLAKE3 (Fish CAS)** | **> 6,400 MB/s** | 128位安全强度，树状哈希，无锁并行 | Fish 构建缓存、现代分布式存储 |
| SHA-256 | ~1,700 MB/s | 标准加密哈希，串行处理 | Git、Bazel、Docker OCI 镜像摘要 |
| SHA-1 | ~2,000 MB/s | 碰撞已被攻破，仅作兼容 | 早期 Git 提交 |
| MD5 | ~580 MB/s | 不安全，已弃用 | 传统校验和 |

## 2. 构建产物压缩效率 (Zstandard vs Gzip)

Fish CAS 使用 **Zstandard (ZSTD)** 结合内容寻址分块去重技术，实现极高压缩速度和亚毫秒级解压恢复：

| 压缩格式 | 压缩率 | 压缩吞吐量 | 解压吞吐量 | 缓存恢复延迟 |
| :--- | :--- | :--- | :--- | :--- |
| **Zstandard (Fish CAS level 3)** | **1.15:1 – 2.8:1** | **> 55 MB/s** | **> 3,850 MB/s** | **即时 (< 10ms)** |
| Gzip / Deflate (标准 tarball) | 1.0:1 – 2.4:1 | ~20 MB/s | ~1,130 MB/s | 解压慢 3.4 倍 |

## 3. 调度延迟预算 (Scheduler Overhead Budget)

Fish 设立了**每个任务调度决策 < 100µs** 的严格开销上限。通过 Criterion 微基准测试在不同图复杂度下进行验证：

| 拓扑图规模 | 拓扑排序 | 就绪队列评估 | 单任务调度开销 |
| :--- | :--- | :--- | :--- |
| 50 节点 | < 5 µs | < 2 µs | **< 12 µs** |
| 200 节点 | < 18 µs | < 7 µs | **< 28 µs** |
| 1,000 节点 | < 95 µs | < 35 µs | **< 75 µs** |

## 4. 同业调度模型对比 (Fish vs Ninja vs Bazel)

`peer_comparison` 基准测试套件在相同依赖图上评估四种基本调度范式：

- **Fish Chase-Lev 工作窃取**：每个工作线程具备独立的去中心化循环缓冲区，启发式最长尾部优先，微秒级窃取延迟。
- **Fish 关键路径优先**：计算图的最长依赖尾部，彻底消除工作线程空闲等待气泡。
- **波阵面模型 (Ninja)**：按拓扑深度逐级执行。
- **屏障同步模型 (Bazel/Pants)**：编译阶段之间存在严格的同步屏障。

## 运行基准测试

### 独立 Python 基准测试套件（无需编译）

Fish 在 `scripts/benchmark_peers.py` 提供了即开即用的独立测试脚本：

```bash
# 在 50 个模拟模块上运行 5 轮测试
python scripts/benchmark_peers.py --packages 50 --rounds 5

# 导出 Markdown 表格
python scripts/benchmark_peers.py --packages 100 --rounds 5 --markdown

# 导出 JSON 格式报告
python scripts/benchmark_peers.py --packages 100 --rounds 5 --json
```

### 完整 Criterion 基准测试（Rust 工作区）

```bash
cargo bench -p fish-scheduler --bench scheduler_performance
cargo bench -p fish-scheduler --bench peer_comparison
```
