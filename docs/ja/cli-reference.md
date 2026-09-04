# Fish CLI コマンドリファレンス

> 🌐 **翻訳と貢献:** このドキュメントをあなたの言語に翻訳または改善したいですか？ [翻訳ガイドライン](TRANSLATION.md) を参照してください。

Fish コマンドラインインターフェースのすべてのコマンド、フラグ、および設定オプションに関する包括的なリファレンスです。

---

## 🧭 基本構文とグローバルオプション

```bash
fish [OPTIONS] <COMMAND>
```

### グローバルフラグ (Global Flags)

| フラグ | 説明 | デフォルト |
|---|---|---|
| `--experimental` | 実験的機能を有効化します。 | `false` |
| `--offline` | ネットワークアクセスを無効化し、オフラインで実行します。 | `false` |
| `-v, --verbose` | 詳細な診断ログと出力を有効化します。 | `false` |
| `-j, --jobs <N>` | 同時実行するワーカーの最大スレッド数。 | CPUコア数 |
| `--no-cache` | キャッシュ（ローカルおよびリモート）の使用を無効化します。 | `false` |
| `--cache-dir <PATH>` | ローカルキャッシュディレクトリのパス（デフォルト: `~/.fish/cache`）。 | システム標準 |

---

## 🛠️ サブコマンド一覧

---

### `fish init`
Fish の設定（`fish.toml`）を初期化し、多言語ワークスペースを検出します。

```bash
fish init [OPTIONS]
```
- `-p, --path <PATH>`: 初期化する対象ディレクトリ。
- `-f, --force`: 既存の設定が存在する場合に上書きします。
- `--describe <DESC>`: 自然言語によるプロジェクト概要（AI支援用）。

---

### `fish new`
テンプレートから新しいプロジェクトまたはパッケージを作成します。

```bash
fish new <NAME> [OPTIONS]
```
- `-t, --template <TEMPLATE>`: テンプレート名（例: `rust`、`ts`、`go`、`polyglot`）。
- `-p, --path <PATH>`: 作成先ディレクトリ。

---

### `fish build`
ワークスペース内のパッケージのビルドタスクを実行します。

```bash
fish build [OPTIONS] [PATH]
```
- `-j, --jobs <N>`: 同時実行ジョブ数の上限。
- `-v, --verbose`: 詳細な実行ログを表示。
- `--no-cache`: キャッシュをスキップ。
- `--sandbox`: サンドボックス内で安全に実行。
- `--apple`: `apple` ハーメティックサンドボックス経由で実行。
- `--profile <FILE>`: Chrome trace JSON プロファイルを生成。
- `--tui`: インタラクティブなターミナルUIを有効化。
- `--remote-cache <URL>`: リモートキャッシュサーバーのURL。
- `--remote-workers <URL>`: 分散タスク実行用のリモートワーカープール。
- `--ram-limit <PCT>`: 利用可能RAMが閾値を下回った際に並行度を制限。
- `--semantic`: ASTレベルのセマンティックキャッシュを有効化。
- `--reflink`: CASからのアーティファクト復元時にCoW（reflink）を使用。
- `--critical-path`: クリティカルパス上のタスクを優先的にスケジューリング。
- `--explain`: タスクが再ビルドされた理由を出力。
- `--otel-endpoint <URL>`: OpenTelemetry コレクターへのトレース送信。

---

### `fish check`
実行可能バイナリをリンクせずに、高速な型チェックおよび静的解析を行います。

```bash
fish check [OPTIONS] [PATH]
```

---

### `fish test`
ワークスペース内のテストスイートを実行します。

```bash
fish test [OPTIONS] [PATH]
```
- `--quarantine-flaky`: 不安定なテスト（flaky test）を自動検出して隔離。
- `--test-threads <N>`: テストの並列実行スレッド数。

---

### `fish clean`
ビルド生成物と一時ファイルを削除し、キャッシュを解放します。

```bash
fish clean [OPTIONS]
```
- `--all`: ローカルCASおよびL1/L2キャッシュ全体を消去。
- `--dry-run`: 削除対象のファイル一覧をシミュレーション表示。

---

### `fish run`
特定の実行可能ターゲットをビルドして実行します。

```bash
fish run -p <PACKAGE> [--bin <BINARY>] [-- <ARGS>...]
```

---

### `fish graph`
ワークスペースの依存関係グラフ（DAG）を出力・可視化します。

```bash
fish graph [OPTIONS]
```
- `--format <FORMAT>`: 出力形式（`dot`, `json`, `mermaid`, `svg`）。
- `--output <FILE>`: グラフをファイルに出力。

---

### `fish watch`
ファイル変更を検知し、自動的にインクリメンタルビルドを実行します。

```bash
fish watch [OPTIONS]
```
- `--debounce <MS>`: 変更イベントのデバウンス時間（デフォルト: 200ms）。

---

### `fish query`
依存関係グラフに対する代数クエリ式を評価します。

```bash
fish query "<EXPRESSION>"
```
- `deps(//pkg)`: 順方向の依存関係。
- `rdeps(//pkg)`: 逆方向の依存関係。
- `allpaths(//a, //b)`: 2つのターゲット間の全パス。
- `somepath(//a, //b)`: 2つのターゲット間の最短パス。

---

### `fish doctor`
環境設定、ツールチェーン、および Fish の構成の健全性を診断します。

```bash
fish doctor [OPTIONS]
```
- `--fix`: 権限、一時ファイル、`fish.toml` の問題を自動修復。
- `--ai`: AIモデルによる高度な診断と修正提案を取得。

---

### `fish why`
特定のターゲットが再ビルドされた理由を説明します。

```bash
fish why <TARGET> [OPTIONS]
```
- `--ask "<QUESTION>"`: 自然言語で再ビルド理由を質問。

---

### `fish fix`
コンパイラ警告やエラーに基づいて、安全な自動修正パッチを適用します。

```bash
fish fix [OPTIONS]
```
- `--diff`: 適用前に Git unified diff をプレビュー表示。
- `--apply`: ソースコードに直接修正を適用。

---

### `fish affected`
Git のコミットやベースブランチと比較して、影響を受けるパッケージを特定します。

```bash
fish affected --base <REF> [--head <REF>]
```

---

### `fish cache`
Content-Addressable Storage (CAS) の管理、クォータ監視、および最適化を行います。

```bash
fish cache <SUBCOMMAND>
```
- `prune`: LRUおよびサイズ閾値に基づき古いブロックを整理。
- `stats`: ヒット率と使用容量の統計情報を表示。
- `verify`: CAS内アーティファクトの整合性を検証。

---

### `fish cost-estimate`
AWS、GCP、Azure 上でのクラウドコンピューティングコストと削減額を見積もります。

```bash
fish cost-estimate [OPTIONS]
```
- `--json`: CI/CD 向けの JSON 形式で出力。

---

### `fish ui`
リアルタイム分析ダッシュボードとDAGグラフビューアーを起動します。

```bash
fish ui [--port <PORT>] [--open]
```

---

### `fish pash`
パス対応セマンティックハッシュ（PASH）の計算結果を表示・検証します。

```bash
fish pash <TARGET>
```

---

### `fish qpc`
クエリパイプラインキャッシュ（QPC）の状態を検査します。

```bash
fish qpc <TARGET>
```

---

### `fish attest` & `fish verify`
アーティファクトに対する Ed25519 デジタル署名と SLSA / in-toto 証明書の生成・検証を行います。

```bash
fish attest --out <ATTESTATION_FILE>
fish verify --attestation <ATTESTATION_FILE>
```

---

### `fish lsp` & `fish daemon`
IDE用LSPサーバーまたはIPC最適化デーモンプロセスを実行します。

```bash
fish lsp
fish daemon [--socket <PATH>]
```
