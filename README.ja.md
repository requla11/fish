<div align="center">

<img src="docs/public/logo.png" alt="Fish Logo" width="180" />

# 🐟 Fish

**超高速・キャッシュ優先のポリグロット・モノレポ向けビルドオーケストレーションシステム**

[![CI](https://github.com/requla11/fish/actions/workflows/dogfood.yaml/badge.svg)](https://github.com/requla11/fish/actions/workflows/dogfood.yaml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Edition](https://img.shields.io/badge/edition-2024-blue.svg)](https://doc.rust-lang.org/edition-guide/rust-2024/index.html)
[![Open in GitHub Codespaces](https://github.com/codespaces/badge.svg)](https://codespaces.new/requla11/fish)

[English](README.md) • [Tiếng Việt](README.vi.md) • [简体中文](README.zh-hans.md) • [繁體中文](README.zh-hant.md) • [日本語](README.ja.md)

</div>

---

**Fish** は、**Rust 2024** でゼロから設計された高性能ビルドオーケストレーションエンジンです。Turborepo のような圧倒的なスピードと直感的な開発体験、そして Bazel のような強力な多言語対応力を兼ね備えています — **Starlark や独自の複雑なビルド DSL は一切不要です**。

Fish はプロジェクト内のツールチェーンを自動検出し、ソースコードを解析して言語をまたぐ依存関係（DAG エッジ）をスマートに導出します。ロックフリーなワークスティーリングプールによる並行タスクスケジューリングと、暗号論的に安全な **BLAKE3** コンテンツアドレス指定ストレージ（CAS）および **Zstandard** 圧縮により、ミリ秒単位の決定論的キャッシュヒットを実現します。

> 💡 **注意：** Fish は既存のコンパイラやパッケージマネージャ（Cargo, Go, npm/pnpm, Python, Clang など）を協調・制御するツールであり、それらを置き換えるものではありません。シェルである [fish-shell](https://fishshell.com) とは無関係であり、名前のみを共有しています。

---

## ✨ 主なハイライト機能

| 機能 | 詳細説明 |
| :--- | :--- |
| ⚡ **サブミリ秒スケジューリング** | Chase-Lev ワークスティーリングキューとクリティカルパス最適化により、100µs 未満でタスクをディスパッチ。 |
| 🌐 **11 以上の言語エコシステム** | Rust, Go, TypeScript/JS, Python, C/C++, Java, .NET, Swift, Dart, Zig, Docker をネイティブサポート。 |
| 🔗 **依存関係の自動推論** | コントラクトファースト：ソース内の参照（`include_str!` や JSON インポートなど）から DAG エッジを自動構成し、手動の `depends_on` 設定は不要。 |
| 💾 **高スループット CAS キャッシュ** | 重複排除 BLAKE3 コンテンツアドレスストレージ、階層型 L1/L2 キャッシュ、高速 ZSTD 圧縮。 |
| 📡 **ゼロ構成 P2P LAN キャッシュ** | クラウドサーバー不要で、ローカル Wi-Fi / LAN を通じてチームメンバー間でビルド成果物を瞬時に共有。 |
| 🛡️ **密閉サンドボックス分離** | マルチプラットフォーム分離：Linux namespaces & Landlock、macOS seatbelt、Windows セキュリティトークン。 |
| 📊 **リアルタイム Web ダッシュボード** | インタラクティブな SVG DAG グラフとテレメトリ指標を備えた Web UI（`fish ui`）を標準搭載。 |

---

## 🚀 クイックインストール

### 1行スクリプトインストール

#### Linux & macOS
```bash
curl -fsSL https://raw.githubusercontent.com/requla11/fish/main/scripts/install.sh | sh
```

#### Windows (PowerShell)
```powershell
irm https://raw.githubusercontent.com/requla11/fish/main/scripts/install.ps1 | iex
```

---

### パッケージマネージャ

| プラットフォーム | パッケージマネージャ | コマンド |
| :--- | :--- | :--- |
| **Windows** | **Scoop** | `scoop install https://raw.githubusercontent.com/requla11/fish/main/packaging/fish.json` |
| **Windows** | **Winget** | `winget install requla11.fish` |
| **macOS** | **Homebrew** | `brew tap requla11/fish https://github.com/requla11/homebrew-fish && brew install fish` |
| **Cargo** | **crates.io / Git** | `cargo install --git https://github.com/requla11/fish.git fish-cli` |

---

## 🏁 クイックスタート

多言語リポジトリのルートディレクトリで次のコマンドを実行します：

```bash
# スマートキャッシュを活用してワークスペース全体を並行ビルド
fish build

# すべての言語のテストスイートを一括実行
fish test

# 監視モード：ファイル変更を検知して自動で差分ビルドとテストを再実行
fish dev

# ビルド成果物のクリーンアップ（--all で ~/.fish/cache も完全初期化）
fish clean --all

# リアルタイム Web ダッシュボードと DAG ビジュアライザを起動
fish ui --open
```

### ポリグロット・デモプロジェクトを試す

**Rust + Go + Python + TypeScript** を組み合わせた実践的なモノレポサンプルが付属しています：

```bash
cd examples/polyglot-demo
fish build
fish graph --format tree
```

ビルド出力例：
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

## 🛠️ サポートしているエコシステム

Fish は以下の 11 主要言語エコシステムを自動検出してオーケストレーションします：

| エコシステム | 検出マニフェスト | デフォルトタスク |
| :--- | :--- | :--- |
| **Rust** | `Cargo.toml` | `cargo check`, `cargo build`, `cargo test` |
| **TypeScript / Node** | `package.json`, `tsconfig.json` | `typecheck`, `build`, `test` |
| **Go** | `go.mod` | `go vet`, `go build`, `go test` |
| **Python** | `pyproject.toml`, `requirements.txt` | 構文チェック, `pytest`, リント |
| **C / C++** | `CMakeLists.txt`, `fish.cc.json` | CMake 構成, ビルド, `ctest` |
| **Java** | `pom.xml`, `build.gradle` | コンパイル, テスト |
| **.NET / C#** | `*.csproj`, `*.sln` | `dotnet build`, `dotnet test` |
| **Swift** | `Package.swift` | `swift build`, `swift test` |
| **Dart / Flutter** | `pubspec.yaml` | `dart analyze`, `dart test` |
| **Zig** | `build.zig` | `zig build`, `zig test` |
| **Docker / OCI** | `Dockerfile`, `docker-compose.yml` | マルチステージビルド, OCI 出力 |

---

## 📋 基本 CLI コマンドリファレンス

直感的で覚えやすいコマンド体系を提供しています：

```text
ビルド & テスト：
  fish build             プロジェクトグラフ内のすべてのターゲットをビルド
  fish check             リンクを行わずに高速な型・構文チェックを実行
  fish test              ワークスペース内の全テストスイートを実行
  fish run [TARGET]      指定したバイナリターゲットをビルドして実行
  fish dev (or watch)    ファイルの変更を監視し、インクリメンタルビルドを実行

解析 & 可視化：
  fish graph             DAG 依存関係をツリー、DOT、または JSON 形式で表示
  fish why <QUERY>       特定のターゲットが再ビルドされた理由を自然言語で照会
  fish ui                リアルタイム Web ダッシュボードとインタラクティブ DAG を開く
  fish doctor            インストールされたツールチェーン、キャッシュの整合性、環境を診断

メンテナンス & クリーン：
  fish clean             ビルドターゲットを削除（-a/--all で ~/.fish/cache を全消去）
  fish fix               AI およびコンパイラからのフィードバックに基づきエラーを自動修正
  fish ci init           最適化された CI/CD ワークフローを自動生成（GitHub Actions, GitLab 等）
  fish affected          Git の変更によって影響を受けたパッケージのみをビルドまたはテスト
```

---

## 🏗️ アーキテクチャとワークスペース構成

Fish は 28 個の Rust クレートで構成されるモジュラーワークスペースです：

```text
crates/
  fish-core/         ワークスペース検出、マニフェストモデル、DAG マージャー
  fish-graph/        依存関係グラフ、トポロジカルソート、クエリアクセス代数
  fish-executor/     プロセス実行、ミドルウェアチェーン、レスポンスファイル
  fish-scheduler/    並行ワークスティーリングスケジューラ、GNU jobserver、動的レーシング
  fish-cache/        フィンガープリントキャッシュ、2フェーズプルーニング、モーフィックハッシュ
  fish-cas/          BLAKE3 + ZSTD 圧縮コンテンツアドレス指定成果物ストレージ
  fish-incremental/  変更検出、AST 推論、差分リビルド説明
  fish-backend-*/    EcosystemBackend を実装する 11 言語・ツールチェーンアダプタ
  fish-worker/       分散実行ワーカーサーバー、ストリーミング VFS プロトコル
  fish-remote-cache/ Ed25519 署名検証付き高スループットリモートキャッシュサーバー
  fish-security/     多層セキュリティ、OSV 脆弱性スキャン、SLSA 出所証明
  fish-cli/          統合 CLI アプリケーション、デーモン IPC、ターミナルレンダリング
submodules/          同梱サブシステム：
  apple/             密閉サンドボックスおよび OS プロセス隔離デーモン
  banana/            P2P Swarm メッシュ、OCI コンテナビルダ、Merkle 台帳
examples/            すぐに実行可能な多言語モノレポサンプル
```

---

## 🌿 ブランチ開発ポリシー

Fish では厳格なブランチ運用を行っています：

```text
dev（アクティブな機能開発、テスト、バグ修正）
  ↓
  ↓ 検証：cargo test --workspace & cargo clippy
  ↓
main（安定した本番リリース専用）
```

- **`dev`** — 日常的な開発、新機能、PR はすべてここにマージされます。
- **`main`** — 安定検証済みの公式リリースタグのみが配置されます。

---

## 🧪 開発環境での検証

コード変更時は以下の検証をパスしていることを確認してください：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

---

## 📖 ドキュメント & コミュニティ

- [アーキテクチャ詳細](ARCHITECTURE.md) — システム内部設計と各コンポーネントの責務。
- [開発環境セットアップ](DEVELOPMENT.md) — ローカル開発環境の構築、デバッグ、ベンチマーク。
- [ロードマップ](ROADMAP.md) — 各マイルストーンの進捗と将来の構想。
- [コントリビューションガイド](CONTRIBUTING.md) — 変更提案の手順や新規バックエンドの追加方法。
- [AI エージェントワークフロー](docs/AI_AGENT_WORKFLOW.md) — AI コーディングエージェントのための開発ガイドライン。

---

## 📄 ライセンス & 免責事項

Fish は [MIT ライセンス](LICENSE) のもとで公開されています。

> **免責事項：** 本プロジェクトは独立したビルドオーケストレーションシステムです。名称に「fish」を含む他の無関係なツールやソフトウェア（`fish-shell`, `fish-image` など）とは一切関係がなく、提携、後援、推奨等を行っているものではありません。
