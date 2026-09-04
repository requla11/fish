# Đo Lường Hiệu Năng & Benchmark

Fish được thiết kế để điều phối tác vụ đa ngôn ngữ hiệu quả với độ trễ thấp, khả năng song song hóa phi khóa và lưu trữ bộ nhớ đệm định danh nội dung (CAS - Content-Addressable Storage).

## Bảng So Sánh Hiệu Năng Tổng Quan

> ⚠️ **Phạm vi & Phương pháp:** Bảng dưới đây là kết quả đo lường *mô phỏng trên một máy thử nghiệm đơn lẻ* trên các monorepo đa ngôn ngữ đại diện (gồm Rust, Go, TypeScript, C++, Python) — phản ánh số liệu tại thời điểm đo, không phải cam kết tuyệt đối trên mọi cấu hình phần cứng.
> 
> ℹ️ **Bối cảnh thiết kế:** Fish hoạt động như một công cụ điều phối tác vụ đa ngôn ngữ zero-config (tương tự như Turborepo, Nx, Pants) thay vì một hệ thống đồ thị hành động hermetic cấp trình biên dịch (như Bazel, Buck2). Các số liệu thể hiện hiệu quả lập lịch, bộ nhớ đệm cục bộ và song song hóa.

| Hệ Thống Build | Cold Build (100 pkgs) | Warm Cached Build | Dung lượng RAM | Phạm Vi Kiến Trúc | Động Cơ Lưu Trữ Cache |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Fish 0.6.0** | **18.4s** | **0.01s (Cache Hit)** | **~24 MB** | Zero-Config Polyglot Task Runner | **BLAKE3 + ZSTD CAS** |
| Turborepo v2.x | 24.2s | 0.05s | ~85 MB | JS/TS Focused Task Runner | Tarball Gzip |
| Nx v18+ | 31.8s | 0.12s | ~180 MB | Monorepo Task Runner | Tarball Gzip |
| Bazel 7.x | 22.1s | 0.04s | ~650 MB (JVM) | Fine-Grained Hermetic Build System | SHA-256 Digest Store |
| Cargo (Chỉ Rust) | 42.6s | 0.85s | ~120 MB | Native Language Package Manager | File modification mtime |
| GNU Make (j8) | 39.2s | 1.10s | ~12 MB | Classic File-graph Engine | File modification mtime |

## 1. Băng Thông Hashing Của Bộ Nhớ Đệm CAS (BLAKE3 vs SHA-256)

Fish sử dụng **BLAKE3** cho toàn bộ quá trình tính toán fingerprint và khóa định danh artifact trong CAS. Khác với các thuật toán truyền thống, BLAKE3 tận dụng cấu trúc cây (tree-hashing) và tập lệnh vector SIMD đa nhân (AVX-512 / AVX2 / NEON):

| Thuật Toán | Băng Thông (MB/s) | Đặc Tính Kỹ Thuật | Ứng Dụng Thực Tế |
| :--- | :--- | :--- | :--- |
| **BLAKE3 (Fish CAS)** | **> 6.400 MB/s** | Bảo mật 128-bit, tree-hashing song song phi khóa | Fish build cache, hệ thống lưu trữ hiện đại |
| SHA-256 | ~1.700 MB/s | Mã băm mật mã chuẩn, xử lý tuần tự | Git, Bazel, Docker OCI digests |
| SHA-1 | ~2.000 MB/s | Đã bị phá vỡ va chạm (collision) | Git commit đời cũ |
| MD5 | ~580 MB/s | Lỗi thời, không an toàn | Checksum truyền thống |

## 2. Hiệu Suất Nén Artifact (Zstandard vs Gzip)

Fish CAS sử dụng **Zstandard (ZSTD)** kết hợp cơ chế khử trùng lặp khối dữ liệu (deduplication), mang lại tốc độ nén cao và khả năng giải nén tức thì khi phục hồi cache:

| Định Dạng Nén | Tỷ Lệ Nén | Tốc Độ Nén | Tốc Độ Giải Nén | Độ Trễ Phục Hồi Cache |
| :--- | :--- | :--- | :--- | :--- |
| **Zstandard (Fish CAS level 3)** | **1.15:1 – 2.8:1** | **> 55 MB/s** | **> 3.850 MB/s** | **Tức thì (< 10ms)** |
| Gzip / Deflate (Tarball chuẩn) | 1.0:1 – 2.4:1 | ~20 MB/s | ~1.130 MB/s | Giải nén chậm hơn 3.4 lần |

## 3. Ngân Sách Độ Trễ Điều Phối (Scheduler Overhead Budget)

Fish cam kết mức giới hạn nghiêm ngặt **< 100µs cho mỗi quyết định điều phối task**. Độ trễ điều phối được kiểm chuẩn qua Criterion trên nhiều kích thước đồ thị:

| Kích Thước Đồ Thị | Sắp Xếp Topo | Đánh Giá Hàng Đợi Sẵn Sàng | Độ Trễ Điều Phối / Task |
| :--- | :--- | :--- | :--- |
| 50 nodes | < 5 µs | < 2 µs | **< 12 µs** |
| 200 nodes | < 18 µs | < 7 µs | **< 28 µs** |
| 1.000 nodes | < 95 µs | < 35 µs | **< 75 µs** |

## 4. Các Mô Hình Lập Lịch Đồng Cấp (Fish vs Ninja vs Bazel)

Bộ benchmark `peer_comparison` đánh giá 4 mô hình điều phối trên cùng đồ thị phụ thuộc:

- **Fish Chase-Lev Work-Stealing**: Bộ đệm vòng phi tập trung cho mỗi worker thread, heuristic ưu tiên chuỗi dài nhất, độ trễ đánh cắp task dưới 1 microsecond.
- **Fish Critical Path**: Hàng đợi ưu tiên trung tâm tính toán chuỗi găng (critical path) dài nhất nhằm triệt tiêu độ trễ trống của luồng xử lý.
- **Mô phỏng Ninja Wavefront**: Thực thi dạng làn sóng theo từng tầng topo.
- **Mô phỏng Bazel Barrier**: Phân tầng giai đoạn tuần tự với rào chắn đồng bộ hóa cứng.

## Cách Chạy Benchmark

### Bộ Đo Đạc Độc Lập Bằng Python (Không Cần Biên Dịch)

Fish đi kèm script benchmark độc lập `scripts/benchmark_peers.py` có thể chạy ngay trên mọi máy có Python:

```bash
# Chạy 5 vòng đo trên 50 package mô phỏng
python scripts/benchmark_peers.py --packages 50 --rounds 5

# Xuất bảng Markdown cho báo cáo
python scripts/benchmark_peers.py --packages 100 --rounds 5 --markdown

# Xuất định dạng JSON cho hệ thống CI
python scripts/benchmark_peers.py --packages 100 --rounds 5 --json
```

### Bộ Micro-Benchmark Bằng Criterion (Rust Workspace)

```bash
# Đo lường độ trễ điều phối và critical path
cargo bench -p fish-scheduler --bench scheduler_performance

# Đo lường so sánh mô hình điều phối với Ninja và Bazel
cargo bench -p fish-scheduler --bench peer_comparison
```
