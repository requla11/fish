# 本番環境デプロイガイド (Deployment)

クラウドおよびエンタープライズ CI/CD 環境での Fish デプロイ手順。

> ⚠️ **ステータスに関する注意:** 以下の構成 2 および 3（`fish-coordinator` による分散クラスター、Kubernetes 自動スケーリング）は**将来の計画**に基づく機能です。現時点ではシングルノードのローカル CI および任意の `fish worker` / `fish cache-server` プロセスのみが利用可能です。[architecture.md](architecture.md) を参照してください。

## デプロイ構成パターン
1. **シングルノード CI**: Runner ホスト上でのローカル CAS キャッシュ運用。
2. **分散クラスター**: `fish-coordinator` によるリモートワーカープールの集中管理。
3. **Kubernetes クラウドネイティブ**: ワーカー Pod の自動スケーリングと Spot インスタンス連携。

## ポート設定
- **Coordinator HTTP/gRPC**: `9090`
- **Worker Gateway**: `9091`
- **P2P Swarm ネットワーク**: `7890` (キャッシュ) / `7891` (コンピュート)
- **OpenTelemetry メトリクス**: `9094`
