# Fish 高度なアルゴリズムとアーキテクチャ革新

> 🌐 **言語ナビゲーション / Language Navigation:**
> [English](../proprietary-tech.md) | [Tiếng Việt](../vi/proprietary-tech.md) | [日本語](proprietary-tech.md) | [简体中文](../zh-hans/proprietary-tech.md) | [繁體中文](../zh-hant/proprietary-tech.md)

---

## ⚡ 概要: Fishの主要なアルゴリズム革新

Fishは、ポリグロットモノレポにおけるスケーリング、キャッシュ無効化、インクリメンタルレイテンシの課題を解決するために設計された4つの特化型アルゴリズムを統合しています。

```
+-------------------------------------------------------------------------------+
|                        FISH コアアルゴリズム革新                              |
+-------------------------------------------------------------------------------+
|  1. Poly-ABI Semantic HyperGraph (PASH)                                       |
|     --> インターフェース境界シンボルの抽出と下流無効化カスケードの遮断        |
|                                                                               |
|  2. Iso-Semantic Morphic Fingerprinting (IS-MFP)                              |
|     --> ローカルとCI環境間のキャッシュミスを排除するDual-Key正規化           |
|                                                                               |
|  3. Speculative Wavelet Pre-Execution (SWPE)                                  |
|     --> 編集イベントのエネルギー分類と事前の依存関係プリウォーミング          |
|                                                                               |
|  4. Virtual Binary Dispatch Table (CAS-VLink)                                 |
|     --> 高速なインクリメンタル反復のためのメモリ内バイナリオーバーレイ        |
+-------------------------------------------------------------------------------+
```

---

## 🔬 1. Poly-ABI Semantic HyperGraph (PASH)
* **配置場所**: `crates/fish-graph`, `crates/fish-core`
* **課題**: 従来のビルドシステムは、公開インターフェース（APIやシグネチャ）が変更されていない場合でも、アップストリームのファイルが変更されるとすべての依存ターゲットを無効化してしまいます。
* **メカニズム**:
  * サポートされている11のバックエンドすべてでエクスポートされた公開インターフェースをスキャンします。
  * 不変の `Symbolic Boundary Hash (SBH)` を計算します。
  * 内部実装のみが変更され `SBH` が同一である場合、PASHは無効化のカスケードを遮断し、不要な再ビルドを防ぎます。

---

## 🧬 2. Iso-Semantic Morphic Fingerprinting (IS-MFP)
* **配置場所**: `crates/fish-cache`, `crates/fish-cas`
* **課題**: ワークスペースのパスの違いや環境の差異により、ローカル端末とCIランナー間でキャッシュヒット率が低下することがあります。
* **メカニズム**:
  * `ExactKey` と `MorphicKey` による **Dual-Key ハッシュアーキテクチャ** を実装。
  * 相対パスの正規化（Windowsのバックスラッシュをスラッシュに統一）および環境ノイズのフィルタリングを実行。
  * 完全一致しない場合はモーフィック一致にフォールバックし、キャッシュの再利用率を最大化します。

---

## 🌊 3. Speculative Wavelet Pre-Execution (SWPE)
* **配置場所**: `crates/fish-incremental`
* **課題**: 手動保存やコマンド実行を待つリアクティブなビルドシステムでは、操作ごとにレイテンシが発生します。
* **メカニズム**:
  * エディタの編集差分をエネルギーレベル（`TrivialWhitespace`、`CommentOnly`、`InternalStatement`、`GlobalInterface`）に分類。
  * 完全なビルドが実行される前に、バックグラウンドメモリでタスクの依存状態を事前準備します。

---

## ⚡ 4. Virtual Binary Dispatch Table (CAS-VLink)
* **配置場所**: `crates/fish-executor`
* **課題**: 小さな変更のたびに完全な再リンクを行うと、反復開発サイクルにおいてオーバーヘッドが生じます。
* **メカニズム**:
  * シンボルアドレスとバイトコードをマッピングするメモリ内 `VirtualBinaryDispatchTable` を保持。
  * 構造化されたランタイムバイナリオーバーレイ（`VLINK_DISPATCH_HEADER_V1`）を生成し、高速なシンボル置換をサポートします。
