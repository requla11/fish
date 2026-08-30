# 比較マトリックス：Fish と他のビルドシステム

Fish は、現代のポリグロット Monorepo 向けに Rust 2024 で設計されたビルドオーケストレーションシステムです。以下は Bazel、Turborepo、Buck2 との客観的な機能比較表です：

| 機能 / 項目 | Fish | Turborepo | Bazel | Buck2 |
| :--- | :--- | :--- | :--- | :--- |
| **実装言語** | Rust 2024 | Go / Rust | Java / C++ | Rust |
| **対応言語** | ネイティブ 11+ 言語ツールチェーン | JS / TS 中心 | ポリグロット (Starlark) | ポリグロット (Starlark) |
| **設定モデル** | 統一 `fish.toml` / 自動検出 | `turbo.json` | `WORKSPACE` + `BUILD.bazel` | `BUCK` ファイル |
| **導入難易度** | 低い（ゼロ構成・自動検出） | 低い | 高い（詳細なルール定義） | 高い（詳細なルール定義） |
| **ハッシュエンジン** | Blake3（並列ツリーハッシュ） | SHA-256 | SHA-256 | Blake3 / SHA-256 |
| **CAS 圧縮** | Zstandard (Zstd) + CoW | Tar.gz / Gzip | Zstd / カスタム | Zstd / カスタム |
| **成果物マテリアライズ** | Reflink / CoW（フォールバック対応） | ファイルコピー | シンボリック/ハードリンク | Reflink / CoW |
| **ブロック重複排除** | FastCDC (16KB - 256KB) | アーカイブ全体 | アーカイブ全体 | チャンク CAS |
| **VFS 解析速度** | インメモリ状態ツリー | ディスク走査 | Inotify / Watchman | Watchman / EdenFS |
| **セマンティック無効化**| AST 公開インターフェースハッシュ | ファイル全体のハッシュ | ヘッダーのみコンパイル | Header / rmeta |
| **AI 診断機能** | ネイティブ IPC + 原因解析 | なし | なし | なし |
| **ダッシュボード** | 組み込み Web GUI & TUI | Vercel Web App | サードパーティ製 | オープンソースコンソール |

---

## アーキテクチャの詳細比較

### Fish vs Turborepo
* **対象言語の範囲:** Turborepo は主に JS/TS 向けに最適化されています。Fish はネイティブのマニフェストから 11+ の言語ツールチェーン（Cargo、Go、CMake、Python、Docker 等）を直接自動検出し、オーケストレーションします。
* **ストレージ効率:** Turborepo は一般的な tarball アーカイブを使用します。Fish は Reflink/CoW と FastCDC 重複排除により、ディスク I/O とネットワーク転送を最小限に抑えます。

### Fish vs Bazel
* **設計思想とトレードオフ:** Bazel は超大規模コードベース向けに、厳格なハーメチックサンドボックスと詳細な `BUILD.bazel` 定義を要求します。Fish はゼロコンフィグのポリグロットタスクランナーとして設計されており、迅速な導入と人間工学（Ergonomics）を重視しています。
* **実行環境:** Bazel は JVM デーモンと専用サンドボックスに依存します。Fish は単一の軽量なネイティブ Rust バイナリとして動作します。

### Fish vs Buck2
* **ワークフロー:** Buck2 は Starlark ルールと外部 Watchman に依存する大規模向けビルドシステムです。Fish はインメモリ VFS と GNU Jobserver プールを内蔵し、追加設定なしですぐに使えるポリグロット体験を提供します。
