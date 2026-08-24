# 生產環境部署指南 (Deployment)

在雲端及企業級 CI/CD 環境中部署 Fish 的完整指南。

> ⚠️ **狀態說明:** 下文中的拓撲架構 2 和 3（基於 `fish-coordinator` 的分散式叢集、Kubernetes 自動擴縮容）屬於**規劃中**的功能。目前僅支援單機本地 CI 以及可選的 `fish worker` / `fish cache-server` 程序。詳見 [architecture.md](architecture.md)。

## 部署拓撲架構
1. **單機 CI 節點**: 在 Runner 本地使用嵌入式 CAS 高速快取。
2. **分散式叢集**: 由 `fish-coordinator` 集中調度遠端 Worker 節點池。
3. **Kubernetes 雲原生**: 自動擴縮容 Worker Pods 並結合 Spot 實例生命週期管理。

## 預設服務連接埠
- **Coordinator HTTP/gRPC**: `9090`
- **Worker Gateway**: `9091`
- **P2P Swarm 網路**: `7890` (快取) / `7891` (計算)
- **OpenTelemetry 指標**: `9094`
