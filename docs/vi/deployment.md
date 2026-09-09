# Hướng Dẫn Triển Khai Sản Xuất (Deployment)

Hướng dẫn toàn diện để triển khai Fish trên môi trường đám mây và hạ tầng CI/CD doanh nghiệp.

> ⚠️ **Lưu ý trạng thái:** Các mô hình 2 và 3 bên dưới (cụm phân tán với `fish-coordinator`, tự động co giãn Kubernetes) mô tả các dịch vụ **theo kế hoạch tương lai**. Hiện tại hệ thống hỗ trợ CI cục bộ đơn nút và các tiến trình opt-in `fish worker` / `fish cache-server`. Xem [architecture.md](architecture.md).

## Các Mô Hình Triển Khai
1. **Máy Chủ CI Độc Lập (Single-Node)**: Sử dụng bộ đệm CAS cục bộ trên máy runner.
2. **Cụm Phân Tán (Distributed Cluster)**: `fish-coordinator` điều phối nhóm máy worker từ xa.
3. **Cloud-Native Kubernetes**: Tự động co giãn pods theo tải thực tế và tối ưu chi phí Spot Instance.

## Cổng Dịch Vụ Mạng
- **Coordinator HTTP/gRPC**: `9090`
- **Worker Gateway**: `9091`
- **Mạng P2P Swarm**: `7890` (Cache) / `7891` (Compute)
- **OpenTelemetry Metrics**: `9094`
