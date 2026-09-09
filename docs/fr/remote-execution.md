# Remote Execution & Distributed Caching

Fish supports high-performance distributed caching and remote execution protocols.

## Remote Execution API (REAPI v2)
Fish ships a Remote Execution API **v2-compatible data model**
(`ReapiDigest`, `ReapiDirectory`, `ReapiFileNode` in
`crates/fish-remote-cache/src/reapi.rs`) transported over HTTP/JSON — not
gRPC. The workspace has no protobuf/gRPC dependencies; see
[go-services.md](go-services.md) for the planned gRPC transport.
```bash
fish build --remote-workers 10.0.0.1:8980,10.0.0.2:8980
```

## P2P Swarm Caching
Enables zero-configuration peer-to-peer artifact sharing over local area networks:
```bash
fish build --swarm
```

## Cloud CAS Backends
Configure S3, GCS, Azure Blob, or Redis storage in `fish.toml`:
```toml
[cache]
enabled = true
remote_url = "s3://my-fish-cache-bucket"
compression = "zstd"
```
