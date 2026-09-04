<div align="center">

<img src="docs/public/logo.png" alt="Fish Logo" width="180" />

# 🐟 Fish

**极速、缓存优先的多语言 Monorepo 构建编排与加速系统**

[![CI](https://github.com/requla11/fish/actions/workflows/dogfood.yaml/badge.svg)](https://github.com/requla11/fish/actions/workflows/dogfood.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/requla11/fish)

[English](README.md) • [Tiếng Việt](README.vi.md) • [简体中文](README.zh-hans.md) • [繁體中文](README.zh-hant.md) • [日本語](README.ja.md)

</div>

---

**Fish** 是一款采用 **Rust 2024** 精心打造的高性能多语言构建编排引擎。它兼具 Turborepo 的极速与直观体验，以及 Bazel 强大的多语言处理能力 — **完全无需学习 Starlark 或复杂的自定义构建 DSL**。

Fish 能够自动发现工具链、解析源代码树以智能推导跨语言依赖边、利用无锁工作窃取（Work-Stealing）池调度任务，并基于高强度 **BLAKE3** 内容寻址存储（CAS）与 **Zstandard** 算法实现全构件缓存。

> 💡 **提示：** Fish 用于协调并调度现有的编译器和包管理器（Cargo、Go、npm/pnpm、Python、Clang 等），并非替代品。本项目与交互式 Shell [fish-shell](https://fishshell.com) 无任何关联。

---

## ✨ 核心特性亮点

| 功能 | 详细描述 |
| :--- | :--- |
| ⚡ **亚毫秒级高效调度** | 基于 Chase-Lev 工作窃取队列与关键路径算法，任务调度分发延迟低于 100µs。 |
| 🌐 **支持 11+ 语言生态** | 原生支持 Rust、Go、TypeScript/JS、Python、C/C++、Java、.NET、Swift、Dart、Zig 与 Docker。 |
| 🔗 **跨语言依赖自动推导** | 契约优先（Contract-first）：引用关系（如 `include_str!`、JSON 导入）自动构建 DAG 边，无需手写 `depends_on`。 |
| 💾 **高吞吐 CAS 缓存** | 基于 BLAKE3 去重的内容寻址存储，结合 L1/L2 分层缓存与 ZSTD 快速压缩。 |
| 📡 **零配置 P2P 局域网缓存** | 支持团队成员在本地 Wi-Fi / 局域网内点对点秒级同步构建构件，无需云端服务器费用。 |
| 🛡️ **密封沙盒隔离** | 多平台沙盒机制：Linux namespaces & Landlock、macOS seatbelt 与 Windows 安全令牌。 |
| 📊 **实时交互式 Web 控制台** | 内置交互式 Web 仪表盘（`fish ui`），提供实时 SVG DAG 依赖图与性能指标遥测。 |

---

## 🚀 快速安装

### 一键脚本安装

#### Linux & macOS
```bash
curl -fsSL https://raw.githubusercontent.com/requla11/fish/main/scripts/install.sh | sh
```

#### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/requla11/fish/main/scripts/install.ps1 | iex
```

---

### 包管理器安装

| 操作系统 | 包管理器 | 命令 |
| :--- | :--- | :--- |
| **Windows** | **Scoop** | `scoop install https://raw.githubusercontent.com/requla11/fish/main/packaging/fish.json` |
| **Windows** | **Winget** | `winget install requla11.fish` |
| **macOS** | **Homebrew** | `brew tap requla11/fish https://github.com/requla11/homebrew-fish && brew install fish` |
| **Cargo** | **crates.io / Git** | `cargo install --git https://github.com/requla11/fish.git fish-cli` |

---

## 🏁 快速上手

在任意多语言项目的根目录下运行：

```bash
# 并行构建整个工作区并启用智能缓存
fish build

# 运行所有语言的测试套件
fish test

# 监听模式：在文件发生变动时自动增量构建与测试
fish dev

# 清理构建产物（添加 --all 可彻底清空本地缓存 ~/.fish/cache）
fish clean --all

# 打开实时交互式 Web 控制台与 DAG 依赖可视化
fish ui --open
```

### 体验多语言示例项目

Fish 自带一个融合了 **Rust + Go + Python + TypeScript** 的契约优先 Monorepo 示例：

```bash
cd examples/polyglot-demo
fish build
fish graph --format tree
```

构建输出示例：
```text
🔗 Inferring cross-language dependencies:
   ↳ go-service → py-worker (Go project references `../py-worker/contracts/events.schema.json`)
   ↳ rust-service → py-worker (Rust project references `../../py-worker/contracts/events.schema.json`)
   ↳ web-frontend → py-worker (TypeScript project references `../../py-worker/contracts/topics.json`)
🔗 Linked 6 cross-project task edge(s) from 3 inference(s)

Build completed successfully.
  Tasks:     7 total (7 cached, 100% cache hit)
  Duration:  0.01s
```

---

## 🛠️ 支持的语言生态系统

Fish 能够原生识别并编排以下 11 个主流开发生态：

| 语言生态 | 识别清单文件 | 默认执行任务 |
| :--- | :--- | :--- |
| **Rust** | `Cargo.toml` | `cargo check`, `cargo build`, `cargo test` |
| **TypeScript / Node** | `package.json`, `tsconfig.json` | `typecheck`, `build`, `test` |
| **Go** | `go.mod` | `go vet`, `go build`, `go test` |
| **Python** | `pyproject.toml`, `requirements.txt` | 语法校验, `pytest`, 代码检查 |
| **C / C++** | `CMakeLists.txt`, `fish.cc.json` | CMake 配置, 构建, `ctest` |
| **Java** | `pom.xml`, `build.gradle` | 编译, 测试 |
| **.NET / C#** | `*.csproj`, `*.sln` | `dotnet build`, `dotnet test` |
| **Swift** | `Package.swift` | `swift build`, `swift test` |
| **Dart / Flutter** | `pubspec.yaml` | `dart analyze`, `dart test` |
| **Zig** | `build.zig` | `zig build`, `zig test` |
| **Docker / OCI** | `Dockerfile`, `docker-compose.yml` | 多阶段镜像构建, OCI 打包 |

---

## 📋 常用 CLI 命令速查

Fish 保持命令行工具简单、直观且易用：

```text
构建与测试：
  fish build             构建项目图中识别出的所有目标
  fish check             极速类型与语法检查（不链接二进制）
  fish test              执行工作区内的所有单元与集成测试
  fish run [TARGET]      构建并直接运行指定的二进制目标
  fish dev (或 watch)    监听源文件改动并自动触发增量重构

分析与观察：
  fish graph             以树状图、DOT 或 JSON 形式打印 DAG 依赖图
  fish why <QUERY>       使用自然语言询问特定目标被重新构建的原因
  fish ui                打开实时 Web 控制台与交互式 DAG 图
  fish doctor            全面诊断本地工具链就绪情况、缓存与环境配置

清理与维护：
  fish clean             清理当前项目构建产物（带 -a/--all 彻底删除 ~/.fish/cache）
  fish fix               基于 AI 与编译器反馈的智能错误诊断与自动修复
  fish ci init           快速生成优化的 CI/CD 配置（GitHub Actions, GitLab 等）
  fish affected          仅针对 Git 变更所影响的相关包进行构建或测试
```

---

## 🏗️ 架构设计与工作区模块划分

本项目采用严谨的模块化 Rust 工作区结构（共 28 个 Crates）：

```text
crates/
  fish-core/         项目自动探测、清单解析与 DAG 合并器
  fish-graph/        依赖有向无环图、拓扑排序与代数查询引擎
  fish-executor/     底层进程执行、中间件链与响应参数文件支持
  fish-scheduler/    并行工作窃取调度器、GNU Jobserver 池与动态竞速
  fish-cache/        多层指纹缓存、双阶段修剪与同态哈希
  fish-cas/          BLAKE3 + ZSTD 高性能内容寻址构件存储
  fish-incremental/  源码变动捕获、AST 依赖推导与构建诊断说明
  fish-backend-*/    11 个主流语言与工具链适配层（实现 EcosystemBackend）
  fish-worker/       分布式远程执行节点与流式虚拟文件系统（VFS）
  fish-remote-cache/ 支持 Ed25519 签名验证的高吞吐远程缓存服务器
  fish-security/     多层次安全合规、OSV 漏洞扫描与 SLSA 产物签名认证
  fish-cli/          统一命令行交互界面、守护进程 IPC 与终端交互呈现
submodules/          配套的安全与网络子系统：
  apple/             Hermetic 密封沙盒与系统进程安全隔离守护进程
  banana/            P2P Swarm 局域网络、OCI 容器构建器与 Merkle 账本
examples/            现成可运行的多语言 Monorepo 实战示例
```

---

## 🌿 分支开发规范（Branch Policy）

Fish 严格遵循双主分支工作流：

```text
dev（活跃特性开发、日常测试、Bug 修复）
  ↓
  ↓ 严格验证：cargo test --workspace & cargo clippy
  ↓
main（生产就绪的高质量稳定发布版本）
```

- **`dev`** — 所有新功能开发、试验性代码与 Pull Request 均合并至此分支。
- **`main`** — 仅包含经过严格验证的高稳定性正式 Release 代码。

---

## 🧪 验证与本地测试

提交代码前请确保通过完整的测试套件检查：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

---

## 📖 扩展文档与社区交流

- [系统架构全景](ARCHITECTURE.md) — 深入了解底层架构设计与各子系统交互。
- [本地开发指南](DEVELOPMENT.md) — 快速搭建本地开发、调试与基准测试环境。
- [项目路线图](ROADMAP.md) — 查看各版本研发里程碑与长远演进计划。
- [贡献指南](CONTRIBUTING.md) — 如何提交高质量代码以及添加新的语言适配器。
- [AI 智能体研发指南](docs/AI_AGENT_WORKFLOW.md) — 面向 AI Coding Agent 的开发最佳实践。

---

## 📄 许可协议与免责声明

Fish 遵循 [MIT 开源许可证](LICENSE)。

> **免责声明：** 本项目是一个完全独立的构建编排系统。名称中带有 "fish" 的其他独立项目（例如 `fish-shell`、`fish-image` 等）与本项目不存在任何归属、赞助或背书关系。
