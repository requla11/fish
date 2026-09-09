# 远程执行与分布式缓存

Fish 支持高性能分布式缓存与远程计算执行协议。

## Remote Execution API (REAPI v2)
Fish 提供 **兼容 REAPI v2 的数据模型**（位于 `crates/fish-remote-cache/src/reapi.rs` 中的 `ReapiDigest`、`ReapiDirectory`、`ReapiFileNode`），通过 HTTP/JSON 传输 — 而非 gRPC。工作区没有 protobuf/gRPC 依赖；关于规划中的 gRPC 传输，请参阅 [go-services.md](go-services.md)。
```bash
fish build --remote-workers 10.0.0.1:8980,10.0.0.2:8980
```

## P2P Swarm 局域网缓存
支持局域网内零配置点对点产物共享：
```bash
fish build --swarm
```

## 云端 CAS 存储
在 `fish.toml` 中配置 S3、GCS、Azure Blob 或 Redis：
```toml
[cache]
enabled = true
remote_url = "s3://my-fish-cache-bucket"
compression = "zstd"
```
