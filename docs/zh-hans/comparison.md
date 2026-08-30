# 对比矩阵：Fish 与其他主流构建系统

Fish 使用 Rust 2024 打造，专为现代多语言 Monorepo 设计。以下是 Fish 与 Bazel、Turborepo 以及 Buck2 的客观横向对比：

| 功能 / 维度 | Fish | Turborepo | Bazel | Buck2 |
| :--- | :--- | :--- | :--- | :--- |
| **核心实现语言** | Rust 2024 | Go / Rust | Java / C++ | Rust |
| **多语言支持** | 原生多语言（11+ 工具链） | 主打 JS / TS | 多语言 (Starlark 规则) | 多语言 (Starlark 规则) |
| **配置模型** | 统一 `fish.toml` / 自动发现 | `turbo.json` | `WORKSPACE` + `BUILD.bazel` | `BUCK` 文件 |
| **上手与配置难度** | 较低（零配置自动识别） | 较低 | 较高（需精细声明规则） | 较高（需精细声明规则） |
| **哈希引擎** | Blake3（并行树状哈希） | SHA-256 | SHA-256 | Blake3 / SHA-256 |
| **CAS 压缩与缓存** | Zstandard (Zstd) + CoW | Tar.gz / Gzip | Zstd / 自定义 | Zstd / 自定义 |
| **产物落地机制** | Reflink / 写时复制（支持回退） | 普通文件复制 | 软链接 / 硬链接 | Reflink / CoW |
| **内容分块引擎** | FastCDC（16KB - 256KB 块去重） | 完整归档文件 | 完整归档文件 | 分块 CAS |
| **VFS 脏文件解析** | 内存快照树 | 磁盘遍历 | Inotify / Watchman 守护进程 | Watchman / EdenFS |
| **语义化缓存失效** | AST 接口签名哈希 (ABI) | 仅文件哈希 | 仅头文件编译 | Header / rmeta 编译 |
| **AI 智能诊断** | 原生 IPC + 故障根因分析 | 无 | 无 | 无 |
| **可视化监控看板** | 原生内置 Web GUI 与 TUI | Vercel 网页看板 | 第三方工具集成 | 开源命令行终端 |

---

## 架构特性详细对比

### Fish 与 Turborepo
* **多语言支持维度:** Turborepo 主要为 JS/TS 生态打造。Fish 原生直接从项目原生清单（`Cargo.toml`, `go.mod`, `CMakeLists.txt` 等）自动发现并协同调度 11+ 种语言工具链。
* **存储与传输效率:** Turborepo 使用标准归档包。Fish 结合 Reflink / CoW 与 FastCDC 内容分块技术，有效降低重复磁盘 I/O 与传输开销。

### Fish 与 Bazel
* **设计定位与权衡:** Bazel 专为超大规模代码库设计，提供极其严苛的文件级密封沙箱，但需要为每个目录编写详尽的 `BUILD.bazel`。Fish 定位为轻量级零配置多语言任务调度器，优先保证开箱即用与开发体验。
* **运行开销:** Bazel 依赖常驻 JVM 守护进程。Fish 为纯 Rust 原生静态二进制，启动迅速且资源占用低。

### Fish 与 Buck2
* **工程复杂度:** Buck2 依赖 Starlark 规则与外部文件监控服务。Fish 内置内存 VFS 与 GNU Jobserver 令牌池，无需复杂的构建工程维护成本即可直接投入使用。
