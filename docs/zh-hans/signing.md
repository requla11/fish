# 产物签名与校验 (Signing & Verification)

Fish 可以对推送到远程缓存的每个构建产物（artifact）进行数字签名，并自动拒绝任何签名验证失败的下载。本文档详细介绍端到端的配置流程。

## 核心概念

| 术语 | 说明 |
|---|---|
| **Seed** | 32 字节随机数（64 个十六进制字符）。用于派生 Ed25519 签名密钥对。请严格保密。 |
| **公钥 (Public key)** | 通过 `fish signing-key` 从 Seed 派生。可公开共享，用于验证签名。 |
| **签名关卡 (Signature gate)** | 远程缓存客户端上的中间件：上传时签名，下载时校验。 |

## 1. 生成签名 Seed

```powershell
# Windows (PowerShell, .NET 加密 RNG)
$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$bytes = New-Object byte[] 32
$rng.GetBytes($bytes)
($bytes | ForEach-Object { $_.ToString("x2") }) -join ""
```

```bash
# macOS / Linux
openssl rand -hex 32
```

请妥善保存 Seed 至密码管理器中。丢失 Seed 意味着失去签名身份；泄露 Seed 意味着他人可以伪造您的构建产物来源。

## 2. 导出公钥

```bash
export FISH_SIGNING_SEED=<your-64-hex-chars>
fish signing-key
# -> 87362bc246e5fe912fa774cfa728cece02545fc3ef7abae394c65e30a2da9455
```

该命令仅输出公钥，Seed 绝不会出现在输出日志中。

## 3. 签名构建产物（生产者端）

```bash
export FISH_SIGNING_SEED=<seed>
fish build            # 上传的产物将自动完成透明签名
```

## 4. 校验构建产物（消费者端）

```bash
export FISH_SIGNING_SEED=<seed>                      # 相同身份，用于重新签名推送
export FISH_TRUSTED_KEYS=87362bc2...9455             # 支持逗号分隔的多个公钥列表
export FISH_SIG_POLICY=refuse                        # 默认策略；使用 "warn" 仅记录告警而不中断构建

fish build           # 验证失败的下载将被直接拒绝
```

策略选项 (Policies)：

- `refuse`（默认）：缺失签名、签名无效或由非受信任密钥签名的下载将导致任务直接失败。
- `warn`：允许下载，但针对违规产物打印告警日志。

## CI/CD 集成

将 `RELEASE_SIGNING_SEED` 配置为仓库 Secret；`.github/workflows/release.yaml` 将在发布版本时使用它签署 SLSA 出处证明（provenance statements）。下游消费者可在其 Runner 的 `FISH_TRUSTED_KEYS` 中固定发布版本的公钥。

## 密钥轮换 (Key Rotation)

1. 生成一个新的 Seed。
2. 将**新公钥**与旧公钥一同添加到 `FISH_TRUSTED_KEYS` 中。
3. 重新构建并重新发布产物。
4. 当所有消费者均不再依赖旧密钥后，从受信任列表中移除旧公钥。
