# アーティファクトの署名と検証 (Signing & Verification)

Fish はリモートキャッシュにプッシュされるすべてのアーティファクトにデジタル署名を行い、署名検証に失敗したダウンロードを自動的に拒否できます。このページでは、そのエンドツーエンドの設定方法を説明します。

## コアコンセプト

| 用語 | 説明 |
|---|---|
| **シード (Seed)** | 32 バイトの乱数（64 桁の 16 進数文字列）。Ed25519 署名鍵ペアを導出します。秘密として保持してください。 |
| **公開鍵 (Public key)** | `fish signing-key` でシードから導出。公開共有可能であり、署名の検証に使用されます。 |
| **署名ゲート (Signature gate)** | リモートキャッシュクライアント上のミドルウェア：アップロード時に署名し、ダウンロード時に検証します。 |

## 1. 署名用シードの生成

```powershell
# Windows (PowerShell, .NET 暗号乱数 RNG)
$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$bytes = New-Object byte[] 32
$rng.GetBytes($bytes)
($bytes | ForEach-Object { $_.ToString("x2") }) -join ""
```

```bash
# macOS / Linux
openssl rand -hex 32
```

シードはパスワードマネージャーに安全にバックアップしてください。シードの紛失は署名アイデンティティの喪失を意味し、漏洩は第三者によるビルド出所の偽造を許すことになります。

## 2. 公開鍵のエクスポート

```bash
export FISH_SIGNING_SEED=<your-64-hex-chars>
fish signing-key
# -> 87362bc246e5fe912fa774cfa728cece02545fc3ef7abae394c65e30a2da9455
```

このコマンドは公開鍵のみを出力し、シードが出力ログに現れることはありません。

## 3. ビルドの署名（プロデューサー側）

```bash
export FISH_SIGNING_SEED=<seed>
fish build            # アップロードされる成果物は自動的に透明に署名されます
```

## 4. アーティファクトの検証（コンシューマー側）

```bash
export FISH_SIGNING_SEED=<seed>                      # プッシュ再署名用の同一アイデンティティ
export FISH_TRUSTED_KEYS=87362bc2...9455             # カンマ区切りのリスト指定が可能
export FISH_SIG_POLICY=refuse                        # デフォルト；失敗せず警告ログのみにする場合は "warn"

fish build           # 検証に失敗したダウンロードは拒否されます
```

ポリシーオプション (Policies)：

- `refuse`（デフォルト）：署名が存在しない、無効である、または信頼されていない鍵で署名されたダウンロードはタスクを失敗させます。
- `warn`：ダウンロードを受け入れますが、該当アーティファクトごとに警告を表示します。

## CI/CD 統合

リポジトリシークレットとして `RELEASE_SIGNING_SEED` を設定します。`.github/workflows/release.yaml` はリリース時にこれを使用して SLSA 出所証明（provenance statements）に署名します。ダウンストリームの利用者は、ランナーの `FISH_TRUSTED_KEYS` にリリースの公開鍵を固定できます。

## 鍵のローテーション (Key Rotation)

1. 新しいシードを生成します。
2. 新しい公開鍵を古い鍵と並べて `FISH_TRUSTED_KEYS` に追加します。
3. アーティファクトをリビルドし、再公開します。
4. 依存するコンシューマーがいなくなった時点で、信頼リストから古い鍵を削除します。
