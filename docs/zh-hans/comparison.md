# 对比矩阵：Fish 与其他构建系统

Fish 是专为现代多语言单体代码仓库（Polyglot Monorepos）打造的构建编排系统，使用 Rust 2024 开发。以下是与 Bazel、Turborepo 和 Buck2 的客观技术对比：

| 功能 / 维度 | Fish | Turborepo | Bazel | Buck2 |
| :--- | :--- | :--- | :--- | :--- |
| **开发语言** | Rust 2024 | Go / Rust | Java / C++ | Rust |
| **多语言支持** | 多语言原生支持 (11+ 工具链) | 专注 JS / TS | 多语言 (Starlark 规则) | 多语言 (Starlark 规则) |
| **配置模型** | `fish.toml` / 零配置自动发现 | `turbo.json` | `WORKSPACE` + `BUILD.bazel` | `BUCK` 规则文件 |
| **配置复杂度** | 极低 / 零配置 | 低 | 高 (需精细声明每个目标) | 高 (需精细声明每个目标) |
| **指纹哈希算法** | Blake3 (多核并行树状哈希) | SHA-256 | SHA-256 | Blake3 / SHA-256 |
| **CAS 产物压缩** | Zstandard (Zstd) + CoW | Tar.gz / Gzip | Zstd / 自定义 | Zstd / 自定义 |
| **产物还原机制** | Reflink / CoW (写时复制，降级拷贝) | 文件全量拷贝 | 符号链接 / 硬链接 | Reflink / CoW |
| **数据分块去重** | FastCDC (16KB - 256KB 内容分块) | 整包归档文件 | 整包归档文件 | 分块 CAS |
| **虚拟文件系统 (VFS)**| 内存快照树 (In-Memory Tree) | 磁盘文件扫描 | Inotify / Watchman 守护进程 | Watchman / EdenFS |
| **语义级失效检测** | AST 接口哈希 (ABI 级别) | 纯文件内容哈希 | 头文件级编译 (Header-only) | 头文件 / rmeta 编译 |
| **智能诊断 (AI)** | 原生 IPC + 启发式错误修复解释 | 无 | 无 | 无 |
| **交互式看板** | 内置 Web GUI + 终端 TUI | Vercel 网页应用 | 第三方控制台 | 开源终端控制台 |

---

## 详细架构剖析

### Fish 对比 Turborepo
* **语言适用范围：** Turborepo 主要面向 JavaScript/TypeScript 生态。Fish 原生扫描并编排 11 种以上原生工具链（Cargo、Go modules、CMake、Python、Docker 等），直接解析各语言标准配置。
* **存储与 I/O 效率：** Turborepo 采用标准 tarball 压缩。Fish 采用文件系统写时复制（Reflink CoW）与 FastCDC 去重分块，最大限度降低磁盘 I/O 和网络传输。

### Fish 对比 Bazel
* **设计理念与权衡：** Bazel 专为需要极严格密封沙箱（Hermetic Sandbox）的超大型代码库设计，每个构建目标必须声明 `BUILD.bazel`。Fish 定位为轻量级零配置任务编排器，优先考虑极速开箱体验与极低资源消耗（Fish 内存占用约 24 MB，而 Bazel JVM 守护进程需 650 MB 以上）。
* **运行时架构：** Bazel 依赖重量级 JVM 守护进程与沙箱包装层。Fish 为单一独立 Rust 本地二进制程序，启动延迟小于 15ms。

### Fish 对比 Buck2
* **工作流易用性：** Buck2 是面向大规模仓库的高性能构建系统，采用 Starlark 规则。Fish 聚焦于开箱即用，内置内存 VFS 与 GNU jobserver 资源池，开发者无需编写复杂配置。

---

## 实证案例研究：Bazel vs Fish（基于 `bazelbuild/examples`）

> ⚠️ **声明 —— 仅供参考：**
> 本案例研究中的实测数据记录于一台代表性 Windows x86_64 开发者工作站（4 核 CPU，约 3.8 GB RAM），测试对象为 Google 官方示例仓库 [`bazelbuild/examples`](https://github.com/bazelbuild/examples)（提交哈希 `3c479f4`）。
> **本数据仅作为技术对比与概念验证参考**。实际生产环境中的构建性能将因硬件规格、磁盘 I/O 速度、网络下载带宽（下载远程工具链规则时）及缓存预热状态而有所不同。Bazel 提供编译器级别的密封隔离保证，需要较高的初始启动开销；而 Fish 专注于提供零配置极速本地执行体验。

### 测试环境设置

针对 `bazelbuild/examples` 中 Go 语言教程的全部三个阶段（`stage1`, `stage2`, `stage3`）进行全面对比测试：
- **缓存完全清理流程：**
  - **Bazel：** 运行 `bazel clean --expunge` 完全清除输出缓存、沙箱并终止后台 JVM 进程。
  - **Fish：** 彻底删除 `.fish/cache` 目录及本地生成目录 `build/`。
- **构建目标范围：** 纯二进制编译产物生成（Bazel 对应 `go_binary`，Fish 对应禁用测试的 `go build`）。

### 实测数据对比表

| 测试模块 | 构建目标 | Bazel 7.4.0 (冷构建) | Bazel 7.4.0 (热缓存命中) | Fish 0.6.0 (冷构建) | Fish 0.6.0 (热缓存命中) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Go Tutorial Stage 1** | `hello` | 165.53s | 23.55s | **1.08s** | **0.00092s (0.9ms)** |
| **Go Tutorial Stage 2** | `print_fortune` | 145.89s | 23.40s | **1.69s** | **0.00095s (0.9ms)** |
| **Go Tutorial Stage 3** | `fortune_test` | 149.68s | 23.70s | **0.99s** | **0.00088s (0.8ms)** |
| **3 个模块合并总计** | **全部 3 个目标** | **461.10s (~7.7 分钟)** | **~70.65s** | **3.76s** | **0.00275s (2.7ms)** |

### 技术成因深度分析

1. **冷构建时间差异分析 (461.10s vs 3.76s)：**
   - **Bazel：** 必须冷启动 Java 虚拟机（JVM），下载 Bazel 7.4 安装包，拉取 `rules_go`，分析 101 个包并配置超过 10,800 个构建目标，在沙箱中编译 `builder.exe` 辅助工具及 Go 标准库。
   - **Fish：** 直接复用本地系统安装的 Go 工具链，初始化时间小于 15ms，省去庞大的沙箱环境下载，直接将任务送入去中心化工作窃取队列。

2. **热缓存命中时间差异分析 (~70.65s vs 0.00275s)：**
   - **Bazel：** 即使代码毫无变动，Bazel 仍需连接 JVM 守护进程，重新求值 Starlark 脚本并比对数千个目标的哈希值。
   - **Fish：** 利用 BLAKE3 树状哈希在微秒级内比对文件元数据与内容指纹。确认无修改后，**100% 命中 CAS 缓存**，3 个项目的总检查时间在 3 毫秒以内完成。
