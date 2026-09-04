# Fish 项目路线图 (Roadmap)

> 🌐 **多语言与贡献:** 想要将此文档翻译或改进为您使用的语言？请参阅 [翻译指南](TRANSLATION.md)。

本文档概述了 Fish 的战略开发路线图，涵盖已完成的里程碑、短期与中期目标、长期愿景以及前沿研究方向。

---

## 🎯 愿景

Fish 旨在成为多语言 Monorepo 和分布式开发环境中最效、最稳健且对开发者最友好的构建编排系统，以单一语言的 **Rust 核心（28 个 crate，Rust 2024，MSRV 1.88+）和 11 个多语言后端**为驱动。可选的 Go/Python 辅助服务及 `proto/` 契约属于前瞻性草案（详见 `ARCHITECTURE.md`）。

我们优先优化的核心目标：

1. **实际构建时间 (Wall-clock time)** — 终端用户唯一能直接感知的指标。
2. **缓存效率** — 命中率，跨机器与跨地域的构建产物复用。
3. **可信度** — 缓存的每个字节都与输入确定性匹配。
4. **工具输出的真实性** — 杜绝虚假诊断与伪造的构建成功。

---

## 🚀 当前里程碑 (v0.2.x) — 已完成

### 第 1 阶段: 核心引擎与多语言基础
- [x] **Rust 核心架构**: 单一语言 Rust 工作区（28 个 crate，resolver = "2"，MSRV 1.88+）- 无 `prost`/`tonic` 依赖；分布式功能直接使用 HTTP/JSON。
- [x] **11 个语言后端**: Rust、Go、TypeScript/Node.js、Python、C/C++、Docker、Java、.NET、Swift、Dart、Zig。
- [x] **Protobuf 契约草案**: `proto/fish/v1/build.proto`、`ai.proto` 和 `coordinator.proto` 设计草案。
- [x] **Blake3 CAS 与两阶段清理**: 带有 Zstandard 压缩的高吞吐内容寻址存储。
- [x] **GNU Jobserver 线程池**: 跨编译器的全局线程令牌分配与动态调度。
- [x] **CI/CD 生成器**: 自动生成 GitHub Actions、GitLab CI、CircleCI、Bitbucket 配置。
- [x] **5 种语言文档**: 已在 GitHub Pages 上线（英文、中文简体、中文繁体、日文、越南文）。

---

## ⚡ 短期目标 (v0.3.x) — 已完成: 开发者体验与协议

### 1. IDE 与编辑器集成
- [x] **VS Code 扩展**: 交互式 DAG 依赖图查看器、一键任务执行与内联诊断。
- [x] **JetBrains 插件套件**: 为 CLion、IntelliJ IDEA 和 Rider 提供 ToolWindow 与 LSP 支持。
- [x] **LSP 语言服务器桥接**: `fish.toml` 实时补全与工作区诊断。

### 2. 高性能 IPC 与服务桥接
- [x] **Daemon IPC 流**: Rust CLI 与 Python 服务间亚毫秒级 JSON-RPC 2.0 通信。
- [x] **gRPC 远程执行 API (REAPI)**: 分布式 Worker 集群的完整 REAPI v2 客户端。
- [x] **eBPF 文件追踪**: Linux 内核级输入/输出捕获与动态依赖分析。

### 3. 智能诊断与 CLI 增强
- [x] **交互式 AI 诊断器**: 主动诊断与自动修复建议（`fish doctor --fix`）。
- [x] **终端 UI (TUI) 增强**: 实时 CPU/RAM 迷你图与任务瀑布流视图。

---

## 🌟 中期目标 (v0.4.x - v0.5.x) — 重点: 云原生分布式基础设施、AI 与成本智能

### 1. 云原生分布式基础设施
- [x] **Kubernetes Operator (Go)**: 用于弹性伸缩 Worker 集群的 CRD（`go/pkg/k8s`）。
- [x] **竞价实例 (Spot) 容错优化**: 节点被抢占时的故障容忍任务迁移。
- [x] **跨地域缓存复制**: P2P 对等 CAS 产物地理分布式同步。

### 2. 机器学习与预测优化
- [x] **构建耗时预测**: 基于 EMA 和历史遥测的耗时估算。
- [x] **不稳定测试 (Flaky Test) 自动隔离**: 统计学非确定性测试检测与隔离。
- [x] **投机性预热 (Speculative Pre-Warming)**: 预测变更并在后台预先编译。

### 3. 遥测、可观测性与团队协作
- [x] **OpenTelemetry 集成**: 贯穿所有构建步骤的 OTLP 分布式追踪。
- [x] **Web 分析仪表盘**: 内部 HTTP 服务器提供构建提速与 Flamegraph。
- [x] **云成本计算器**: AWS/GCP/Azure 实时成本估算与节约报告（`fish cost-estimate`）。
- [x] **分布式追踪聚合**: 合并来自所有 Worker 的 Span。
- [x] **构建性能衰退告警**: 自动检测 PR 与基准分支间的耗时退化。

---

## 🧭 v0.6.x — 重点: 可靠性、气密性与供应链安全

- [x] **气密性工具链下载器**: 带有 SHA-256 校验的工具链自动拉取。
- [x] **工具链锁定文件**: `fish.lock` 版本确定性锁定。
- [x] **离线模式保障**: `--offline` 标志保证完全离线确定性。
- [x] **追踪重放 (Trace Replay)**: 记录并重放构建执行以验证气密性。
- [x] **逐比特重现性认证**: 产物目录 BLAKE3 哈希比对。
- [x] **环境漂移检测器**: 告警编译器及操作系统环境波动。
- [x] **沙箱策略配置文件**: `strict`/`default`/`trusted` 权限分级。
- [x] **产物签名验证门禁**: 拒绝未经验证的远程 CAS 产物。
- [x] **依赖漏洞审计集成**: 接入 RustSec/OSV 漏洞源（`fish-security/src/osv.rs`）。

---

## 🤖 v0.7.x — 重点: 原生 AI 构建体验

- [x] **编译器对齐的自动修复**: `fish fix` 依据编译器输出应用安全补丁。
- [x] **自然语言构建查询**: `fish why --ask` 基于真实数据解答构建原因。
- [x] **自适应资源调控**: 基于 P90 内存预测动态调整工作池容量。
- [x] **智能测试筛选 (Test Selection)**: 依据依赖影响图跳过无关测试。
- [x] **构建时序数据存储**: 本地 SQLite WAL 记录指标。

---

## 🏛️ 长期愿景 (v1.0+) — 重点: 企业级与零信任架构

- [x] **MicroVM 硬件隔离**: 基于 Firecracker 的轻量级虚拟机隔离执行。
- [ ] **企业身份认证 (SSO / OIDC)**: RBAC 权限控制与审计日志。
- [ ] **密码学供应链凭证 (SLSA Level 3)**: 生成防篡改 in-toto 证明。
- [x] **高可用协调器 (HA Coordinator)**: Go 控制面中的 Raft 一致性协议。
- [x] **多租户缓存隔离**: 按团队配额划分 CAS 命名空间。
- [x] **多语言 AST 子树缓存**: 函数粒度的增量编译。
- [x] **全局 P2P 网格分发**: 受 BitTorrent 启发的产物共享。

---

## 🚀 前沿研究方向 (v2.0 Moonshots)

- [ ] **编译器查询钩子**: 与 rustc/tsc/clang 的深度增量单元集成。
- [x] **自愈构建 (Self-Healing Builds)**: 失败时自动进行 git bisect 并准备修复 PR。
- [x] **碳感知调度 (Carbon-Aware Scheduling)**: 调度工作负载至低碳排放时段。
- [ ] **全球构建网格联盟**: 跨组织匿名共享通用依赖 CAS 块。
- [ ] **自然语言构建编写**: 从自然语言描述自动生成类型安全的 `fish.yaml`。
