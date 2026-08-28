# Fish 独自の技術と次世代アルゴリズム

> 🌐 **言語ナビゲーション:**
> [English](../../PROPRIETARY_TECH.md) | [Tiếng Việt](../vi/proprietary-tech.md) | [简体中文](../zh-Hans/proprietary-tech.md) | [繁體中文](../zh-Hant/proprietary-tech.md) | [日本語](proprietary-tech.md)

---

## ⚡ 概要: Fish Quantum Polyglot Core (QPC)

Fish は、多言語モノレポおよび分散ビルドシステムのスケーリングと無効化のボトルネックを解決するために特別に設計された、4 つの独自のアルゴリズムを開発しています。

```
+-------------------------------------------------------------------------------+
|                      FISH QUANTUM POLYGLOT CORE (QPC)                         |
+-------------------------------------------------------------------------------+
|  1. Poly-ABI Semantic HyperGraph (PASH)                                       |
|     --> 言語間の公開境界抽出と無効化カスケードの遮断                          |
|                                                                               |
|  2. Iso-Semantic Morphic Fingerprinting (IS-MFP)                              |
|     --> キャッシュの断崖絶壁 (Cache Cliff) を解消するデュアルキー CAS エンジン|
|                                                                               |
|  3. Speculative Wavelet Pre-Execution (SWPE)                                  |
|     --> リアルタイム LSP トークン駆動型の投機的マイクロコンパイル             |
|                                                                               |
|  4. CAS-VLink (Virtual Jump-Table Splicer)                                    |
|     --> システムリンカーをバイパスするゼロコピー仮想バイナリスプライサー      |
+-------------------------------------------------------------------------------+
```

---

## 🔬 1. Poly-ABI Semantic HyperGraph (PASH)
* **ステータス**: 開発中 (`crates/fish-graph`, `crates/fish-core`)
* **課題**: 既存のビルドシステムは、公開インターフェース (ABI) に変更がなくても、上流のファイル変更ですべての下流ターゲットを無効化します。
* **仕組み**: 11 の言語バックエンドすべてで公開境界 (PIB) を抽出し、`Symbolic Boundary Hash (SBH)` を計算します。内部ロジックのみの変更時は無効化の伝播を遮断します。

---

## 🧬 2. Iso-Semantic Morphic Fingerprinting (IS-MFP)
* **ステータス**: 開発中 (`crates/fish-cache`, `crates/fish-cas`)
* **課題**: パスの違いや環境エントロピーにより、ローカルと CI 間でキャッシュヒット率が 0% に低下します。
* **仕組み**: `ExactKey` と `MorphicKey` によるデュアルキーハッシュを実装し、パスやタイムスタンプの差異を正規化して 95% 以上のキャッシュ再利用率を達成します。

---

## 🌊 3. Speculative Wavelet Pre-Execution (SWPE)
* **ステータス**: 開発中 (`crates/fish-scheduler`, `crates/fish-incremental`)
* **課題**: 受動的なビルドシステムは保存キーが押されるまで待機し、開発者の待ち時間を増加させます。
* **仕組み**: `Fish LSP Bridge` から入力差分を受信し、アイドル CPU トークンを活用して型推論と中間コードを事前生成します。

---

## ⚡ 4. CAS-VLink (Virtual Jump-Table Splicer)
* **ステータス**: 開発中 (`crates/fish-executor`, `crates/fish-cas`)
* **課題**: システムリンカー (`ld`, `lld`) は大規模なバイナリ構築時間の 40〜60% を消費します。
* **仕組み**: 出力バイナリ内に仮想ディスパッチテーブル (VBDT) を構築し、変更されたセグメントをゼロコピーでスプライスしてリンク時間を 10〜50 倍高速化します。
