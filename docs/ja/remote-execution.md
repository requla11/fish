# リモート実行と分散キャッシュ

Fish は高パフォーマンスな分散キャッシュおよびリモートビルド実行プロトコルをサポートします。

## Remote Execution API (REAPI v2)
Fish は **REAPI v2 互換のデータモデル**（`crates/fish-remote-cache/src/reapi.rs` 内の `ReapiDigest`, `ReapiDirectory`, `ReapiFileNode`）を提供し、gRPC ではなく HTTP/JSON 経由で転送します。ワークスペースには protobuf/gRPC の依存関係はありません。gRPC 転送の計画については [go-services.md](go-services.md) を参照してください。
```bash
fish build --remote-workers 10.0.0.1:8980,10.0.0.2:8980
```

## P2P Swarm キャッシュ
ローカルネットワーク内でのゼロコンフィグ P2P アーティファクト共有：
```bash
fish build --swarm
```

## クラウド CAS バックエンド
`fish.toml` で S3、GCS、Azure Blob、Redis を設定：
```toml
[cache]
enabled = true
remote_url = "s3://my-fish-cache-bucket"
compression = "zstd"
```
