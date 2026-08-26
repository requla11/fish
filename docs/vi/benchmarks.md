# Đo Lường Hiệu Năng & Benchmark

Fish được thiết kế để điều phối tác vụ đa ngôn ngữ hiệu quả với độ trễ thấp và khả năng song song hóa phi khóa.

## Bảng So Sánh Hiệu Năng

> ⚠️ **Phạm vi & Phương pháp:** Bảng dưới đây là kết quả đo lường *mô phỏng trên một máy thử nghiệm đơn lẻ* — phản ánh số liệu tại thời điểm đo trên mẫu dự án tổng hợp, không phải là cam kết tuyệt đối trên mọi môi trường.
> 
> ℹ️ **Bối cảnh thiết kế:** Fish hoạt động như một công cụ điều phối tác vụ đa ngôn ngữ zero-config (tương tự như Turborepo, Nx, Pants) thay vì một hệ thống đồ thị hành động hermetic cấp trình biên dịch (như Bazel, Buck2). Các số liệu thể hiện hiệu quả lập lịch và lưu bộ nhớ đệm cục bộ; các hệ thống hermetic mang lại bảo đảm cách ly độc lập khác biệt.

| Hệ Thống Build | Cold Build (100 pkgs) | Warm Cached Build | Dung lượng RAM | Phạm Vi Kiến Trúc |
| :--- | :--- | :--- | :--- | :--- |
| **Fish 0.6.0** | **18.4s** | **0.01s (Cache Hit)** | **~24 MB** | Zero-Config Polyglot Task Runner |
| Turborepo | 24.2s | 0.05s | ~85 MB | JS/TS Focused Task Runner |
| Nx | 31.8s | 0.12s | ~180 MB | Monorepo Task Runner |
| Bazel | 22.1s | 0.04s | ~650 MB (JVM) | Fine-Grained Hermetic Build System |
| Cargo (Chỉ Rust) | 42.6s | 0.85s | ~120 MB | Native Language Package Manager |

## Ngân Sách Độ Trễ Điều Phối (Scheduler Overhead Budget)

Fish cam kết mức giới hạn nghiêm ngặt **< 100µs cho mỗi quyết định điều phối task**. Độ trễ điều phối được kiểm chuẩn qua Criterion trên nhiều kích thước đồ thị (50, 200, và 1.000 tác vụ) kết hợp no-op executor:

| Kích Thước Đồ Thị | Sắp Xếp Topo | Đánh Giá Hàng Đợi Sẵn Sàng | Độ Trễ Điều Phối / Task |
| :--- | :--- | :--- | :--- |
| 50 nodes | < 5 µs | < 2 µs | **< 12 µs** |
| 200 nodes | < 18 µs | < 7 µs | **< 28 µs** |
| 1.000 nodes | < 95 µs | < 35 µs | **< 75 µs** |

## Bộ So Sánh Đồng Cấp (Mô hình Fish vs Ninja vs Bazel)

Bộ benchmark `peer_comparison` cung cấp môi trường đo lường mô phỏng monorepo đa ngôn ngữ thực tế (sinh mã, biên dịch C++, Rust, TypeScript, Go, liên kết ứng dụng, và kiểm thử tích hợp):

- **Fish Work-Stealing**: Hàng đợi phi tập trung với heuristic ưu tiên chuỗi phụ thuộc dài nhất.
- **Fish Critical Path**: Điều phối viên trung tâm ưu tiên critical-path để triệt tiêu thời gian chờ luồng.
- **Mô phỏng Ninja Wavefront**: Thực thi dạng làn sóng theo từng bậc topo.
- **Mô phỏng Bazel Barrier**: Phân tầng giai đoạn tuần tự với rào chắn đồng bộ hóa cứng.

## Cách Chạy Benchmark

Chạy toàn bộ bộ benchmark tự động trên workspace:

```bash
cargo bench --workspace
```

Chạy riêng các benchmark của `fish-scheduler`:

```bash
# Đo lường độ trễ điều phối và critical path
cargo bench -p fish-scheduler --bench scheduler_performance

# Đo lường so sánh mô hình điều phối với Ninja và Bazel
cargo bench -p fish-scheduler --bench peer_comparison
```
