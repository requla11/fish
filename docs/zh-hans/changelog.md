# 更新日志与版本历史

Fish 项目的所有重要变更均记录于此。

格式基于 [Keep a Changelog](https://keepachangelog.com/en/1.0.0/)，并遵循 [语义化版本规范](https://semver.org/spec/v2.0.0.html)。

## [v0.6.0] - 2026-08-25

### 新增
- **跨语言 Protobuf 协议**：在 Rust、Go 和 Python 之间实现二进制 Google Protocol Buffers wire 编解码，无需笨重的外部编译器依赖。
- **Wasm 插件引擎与安全审计**：沙箱化 WebAssembly 插件支持、权限能力审计（`fish plugin audit`）及 Ed25519 密码学签名验证。
- **ZSTD 内容寻址存储 (CAS)**：极速 BLAKE3 树状哈希与多线程 Zstandard 压缩，构建确定性 L1/L2 缓存。
- **11 种多语言生态后端**：针对 Rust、Go、TypeScript/Node、Python、C/C++、Docker、Java、.NET、Swift、Dart 和 Zig 的原生零配置支持。
- **自适应并行与工作窃取**：基于 Chase-Lev 双端队列去中心化工作窃取调度，结合关键路径启发式优化与 RAM 内存防颠簸控制。

### 优化
- 依赖环检测重构，提供完整闭环路径诊断而非笼统报错。
- 本地缓存命中时自动解压还原任务声明的产物文件至磁盘。

## [v0.5.0] - 2026-08-24

### 新增
- **5 语言文档门户**：基于 VitePress 的完整文档系统，支持英语、越南语、简体中文、繁体中文和日语。
- **分布式协调器 (Go)**：高吞吐工作节点协调器，具备心跳追踪与 HTTP/Protobuf 任务分派能力。
- **AI 错误分析与修复 (Python)**：基于子进程通道的编译器错误诊断解析与预测性预热。

## [v0.3.0] - 2026-08-21

### 新增
- **IDE 扩展**：官方 VS Code 扩展与 Language Server Protocol (`fish lsp`) 语言服务器集成。
- **交互式 TUI 控制台**：实时多线程构建进度、CPU/RAM 占用率与瀑布流可视化。
- **eBPF 动态追踪**：在 Linux 内核层捕获文件访问与动态依赖。

## [v0.2.0] - 2026-08-10

### 新增
- **三引擎核心架构**：Rust 2024 核心引擎配合 Go 分布式网络与 Python AI 优化服务。
- **指纹缓存引擎**：基于 BLAKE3 的超高吞吐任务指纹与增量变更检测。
- **GNU Jobserver 资源池**：全局并发度治理，防止编译资源耗尽。

## [v0.1.0] - 2026-08-01

### 新增
- Fish 首个实验性版本发布，提供 Rust 与 TypeScript 构建支持。
