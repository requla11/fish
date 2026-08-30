# 產物簽名與驗證 (Signing & Verification)

Fish 可以對推送到遠端快取的每個構建產物（artifact）進行數位簽名，並自動拒絕任何簽名驗證失敗的下載。本文件詳細介紹端到端的設定流程。

## 核心概念

| 術語 | 說明 |
|---|---|
| **Seed** | 32 位元組隨機數（64 個十六進位字元）。用於衍生 Ed25519 簽名金鑰對。請嚴格保密。 |
| **公鑰 (Public key)** | 透過 `fish signing-key` 從 Seed 衍生。可公開共享，用於驗證簽名。 |
| **簽名閘門 (Signature gate)** | 遠端快取客戶端上的中介軟體：上傳時簽名，下載時驗證。 |

## 1. 產生簽名 Seed

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

請妥善保存 Seed 至密碼管理工具中。丟失 Seed 意味著失去簽名身分；洩漏 Seed 意味著他人可以偽造您的構建產物來源。

## 2. 匯出公鑰

```bash
export FISH_SIGNING_SEED=<your-64-hex-chars>
fish signing-key
# -> 87362bc246e5fe912fa774cfa728cece02545fc3ef7abae394c65e30a2da9455
```

該命令僅輸出公鑰，Seed 絕不會出現在輸出日誌中。

## 3. 簽名構建產物（生產者端）

```bash
export FISH_SIGNING_SEED=<seed>
fish build            # 上傳的產物將自動完成透明簽名
```

## 4. 驗證構建產物（消費者端）

```bash
export FISH_SIGNING_SEED=<seed>                      # 相同身分，用於重新簽名推送
export FISH_TRUSTED_KEYS=87362bc2...9455             # 支援逗號分隔的多個公鑰清單
export FISH_SIG_POLICY=refuse                        # 預設策略；使用 "warn" 僅記錄警告而不中斷構建

fish build           # 驗證失敗的下載將被直接拒絕
```

策略選項 (Policies)：

- `refuse`（預設）：缺失簽名、簽名無效或由非受信任金鑰簽名的下載將導致任務直接失敗。
- `warn`：允許下載，但針對違規產物列印警告日誌。

## CI/CD 整合

將 `RELEASE_SIGNING_SEED` 設定為存放庫 Secret；`.github/workflows/release.yaml` 將在發布版本時使用它簽署 SLSA 出處證明（provenance statements）。下游消費者可在其 Runner 的 `FISH_TRUSTED_KEYS` 中固定發布版本的公鑰。

## 金鑰輪替 (Key Rotation)

1. 產生一個新的 Seed。
2. 將**新公鑰**與舊公鑰一同新增到 `FISH_TRUSTED_KEYS` 中。
3. 重新構建並重新發布產物。
4. 當所有消費者均不再依賴舊金鑰後，從受信任清單中移除舊公鑰。
