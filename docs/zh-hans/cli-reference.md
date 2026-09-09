# Fish CLI 命令行参考手册

> 🌐 **多语言与贡献:** 想要将此文档翻译或改进为您使用的语言？请参阅 [翻译指南](TRANSLATION.md)。

Fish 命令行界面的完整参考手册，涵盖所有可用子命令、选项标志及配置。

---

## 🧭 基本语法与全局选项

```bash
fish [OPTIONS] <COMMAND>
```

### 全局标志 (Global Flags)

| 标志 (Flag) | 说明 | 默认值 |
|---|---|---|
| `--experimental` | 启用实验性功能。 | `false` |
| `--offline` | 禁用网络访问，纯离线执行并快速失败。 | `false` |
| `-v, --verbose` | 启用详细诊断日志与执行输出。 | `false` |
| `-j, --jobs <N>` | 最大并行工作线程数。 | CPU 核心数 |
| `--no-cache` | 禁用本地和远程缓存。 | `false` |
| `--cache-dir <PATH>` | 本地缓存目录路径（默认: `~/.fish/cache`）。 | 系统默认 |

---

## 🛠️ 子命令完整列表

---

### `fish init`
初始化 Fish 配置文件（`fish.toml`）并扫描多语言工作区。

```bash
fish init [OPTIONS]
```
- `-p, --path <PATH>`: 初始化的目标目录。
- `-f, --force`: 强制覆盖已存在的配置文件。
- `--describe <DESC>`: 使用自然语言描述项目结构（用于 AI 辅助配置）。

---

### `fish new`
基于内置模板创建新项目或子包。

```bash
fish new <NAME> [OPTIONS]
```
- `-t, --template <TEMPLATE>`: 模板名称（如: `rust`、`ts`、`go`、`polyglot`）。
- `-p, --path <PATH>`: 目标路径。

---

### `fish build`
执行工作区各包的构建任务。

```bash
fish build [OPTIONS] [PATH]
```
- `-j, --jobs <N>`: 限制并行任务数。
- `-v, --verbose`: 打印详细构建步骤。
- `--no-cache`: 跳过缓存。
- `--sandbox`: 在安全沙箱中执行任务。
- `--apple`: 通过 `apple` 密闭沙箱执行。
- `--profile <FILE>`: 输出 Chrome trace JSON 性能分析文件。
- `--tui`: 启用交互式终端 UI。
- `--remote-cache <URL>`: 远程缓存服务器地址（HTTP 或 gRPC REAPI）。
- `--remote-workers <URL>`: 远程分布式 Worker 集群。
- `--ram-limit <PCT>`: 当可用内存低于该百分比时自动降低并行度。
- `--semantic`: 启用 AST 级语义缓存。
- `--reflink`: 从 CAS 恢复产物时使用写时复制 (reflink)。
- `--critical-path`: 优先调度关键路径上的任务。
- `--explain`: 打印各任务被重新构建的原因。
- `--otel-endpoint <URL>`: 导出 OpenTelemetry 追踪至 OTLP 收集器。

---

### `fish check`
仅执行快速类型检查与静态分析，无需链接生成完整二进制文件。

```bash
fish check [OPTIONS] [PATH]
```

---

### `fish test`
运行工作区中的所有测试套件。

```bash
fish test [OPTIONS] [PATH]
```
- `--quarantine-flaky`: 自动检测并隔离不稳定测试 (flaky test)。
- `--test-threads <N>`: 测试并发线程数。

---

### `fish clean`
清理构建临时文件并释放缓存空间。

```bash
fish clean [OPTIONS]
```
- `--all`: 清空本地 CAS 与 L1/L2 缓存。
- `--dry-run`: 预览将要删除的文件列表而不实际删除。

---

### `fish run`
构建并运行指定的可执行二进制目标。

```bash
fish run -p <PACKAGE> [--bin <BINARY>] [-- <ARGS>...]
```

---

### `fish graph`
导出并可视化工作区依赖有向无环图 (DAG)。

```bash
fish graph [OPTIONS]
```
- `--format <FORMAT>`: 导出格式（`dot`, `json`, `mermaid`, `svg`）。
- `--output <FILE>`: 写入图表到指定文件。

---

### `fish watch`
监听文件修改并自动触发增量增效构建。

```bash
fish watch [OPTIONS]
```
- `--debounce <MS>`: 文件变更去抖动缓冲时间（默认: 200ms）。

---

### `fish query`
对依赖图执行代数表达式查询。

```bash
fish query "<EXPRESSION>"
```
- `deps(//pkg)`: 目标包的正向依赖。
- `rdeps(//pkg)`: 目标包的反向依赖。
- `allpaths(//a, //b)`: 两个目标之间的所有路径。
- `somepath(//a, //b)`: 两个目标之间的最短路径。

---

### `fish doctor`
诊断开发环境、工具链及 Fish 配置的健康状态。

```bash
fish doctor [OPTIONS]
```
- `--fix`: 自动修复权限、孤立临时文件及 `fish.toml` 配置问题。
- `--ai`: 调用 AI 引擎提供深度诊断与修复方案。

---

### `fish why`
解释某个目标包被重新构建的具体原因。

```bash
fish why <TARGET> [OPTIONS]
```
- `--ask "<QUESTION>"`: 使用自然语言询问重构原因。

---

### `fish fix`
依据编译器错误与警告输出应用安全的自动修复补丁。

```bash
fish fix [OPTIONS]
```
- `--diff`: 应用前预览 Git unified diff。
- `--apply`: 直接将修复代码应用至源码文件。

---

### `fish affected`
比对 Git 提交或基准分支，定位受变更影响的包列表。

```bash
fish affected --base <REF> [--head <REF>]
```

---

### `fish cache`
管理、统计与优化内容寻址存储 (CAS)。

```bash
fish cache <SUBCOMMAND>
```
- `prune`: 基于 LRU 与配额清理过期数据块。
- `stats`: 查看缓存命中率与磁盘占用。
- `verify`: 校验 CAS 产物的哈希完整性。

---

### `fish cost-estimate`
估算在 AWS、GCP、Azure 上的计算资源消耗与节约金额。

```bash
fish cost-estimate [OPTIONS]
```
- `--json`: 输出 JSON 格式供 CI/CD 流水线解析。

---

### `fish ui`
启动实时性能分析与 DAG 图表 Web 控制台。

```bash
fish ui [--port <PORT>] [--open]
```

---

### `fish pash`
计算并检查路径感知语义哈希 (Path-Aware Semantic Hashing)。

```bash
fish pash <TARGET>
```

---

### `fish qpc`
检查查询流水线缓存 (Query Pipeline Cache) 状态。

```bash
fish qpc <TARGET>
```

---

### `fish attest` & `fish verify`
生成与验证构建产物的 Ed25519 密码学签名及 SLSA / in-toto 凭据。

```bash
fish attest --out <ATTESTATION_FILE>
fish verify --attestation <ATTESTATION_FILE>
```

---

### `fish lsp` & `fish daemon`
启动 IDE 语言服务器或后台常驻 IPC 守护进程。

```bash
fish lsp
fish daemon [--socket <PATH>]
```
