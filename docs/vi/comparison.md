# Bảng So Sánh: Fish và Các Hệ Thống Build Khác

Fish được thiết kế và xây dựng bằng Rust 2024 dành cho các monorepo đa ngôn ngữ hiện đại. Dưới đây là bảng so sánh kỹ thuật khách quan với Bazel, Turborepo và Buck2:

| Tính năng / Khía cạnh | Fish | Turborepo | Bazel | Buck2 |
| :--- | :--- | :--- | :--- | :--- |
| **Ngôn ngữ phát triển** | Rust 2024 | Go / Rust | Java / C++ | Rust |
| **Hỗ trợ ngôn ngữ** | Đa ngôn ngữ (11+ toolchains) | Tập trung JS / TS | Đa ngôn ngữ (Starlark) | Đa ngôn ngữ (Starlark) |
| **Mô hình cấu hình** | `fish.toml` tự động nhận diện | `turbo.json` | `WORKSPACE` + `BUILD.bazel` | `BUCK` files |
| **Độ phức tạp thiết lập** | Thấp / Zero-config | Thấp | Cao (cần cấu hình chi tiết) | Cao (cần cấu hình chi tiết) |
| **Bộ băm Fingerprint** | Blake3 (băm song song dạng cây) | SHA-256 | SHA-256 | Blake3 / SHA-256 |
| **Nén & CAS Cache** | Zstandard (Zstd) + CoW | Tar.gz / Gzip | Zstd / Custom | Zstd / Custom |
| **Xuất Artifact ra đĩa** | Reflink / CoW (dự phòng copy) | File copy | Symlinks / Hardlinks | Reflink / CoW |
| **Phân mảnh khối** | FastCDC (16KB - 256KB) | Toàn bộ tệp nén | Toàn bộ tệp nén | Chunked CAS |
| **Tốc độ VFS** | Cây In-Memory RAM | Quét filesystem | Daemon Inotify/Watchman | Watchman / EdenFS |
| **Semantic Invalidation** | AST Interface Hash (ABI) | Băm toàn bộ file | Header-only compile | Header / rmeta compile |
| **Chuẩn đoán AI** | Tích hợp IPC & Explain | Không có | Không có | Không có |
| **Giao diện Dashboard** | Tích hợp sẵn Web GUI & TUI | Vercel Web App | Phụ thuộc bên thứ ba | Open-source console |

---

## Phân Tích Kiến Trúc Kỹ Thuật

### Fish và Turborepo
* **Phạm vi ngôn ngữ:** Turborepo phục vụ chủ yếu hệ sinh thái JavaScript/TypeScript. Fish tự động nhận diện và điều phối 11+ chuỗi công cụ (Rust, Go, C++, Python, Docker, v.v.) trực tiếp từ các tệp manifest gốc của dự án.
* **Tốc độ I/O:** Turborepo sử dụng nén tarball tiêu chuẩn. Fish dùng Reflink CoW và phân mảnh khối FastCDC để giảm thiểu I/O đĩa và băng thông mạng không cần thiết.

### Fish và Bazel
* **Triết lý thiết kế & Đánh đổi:** Bazel được xây dựng cho các kho mã khổng lồ với yêu cầu hermetic sandbox nghiêm ngặt và quy tắc `BUILD.bazel` chi tiết cho từng target. Fish định vị là một công cụ điều phối tác vụ đa ngôn ngữ zero-config, ưu tiên tốc độ tiếp cận và dễ sử dụng thay vì xây dựng đồ thị hành động quá chi tiết ở mức từng file.
* **Môi trường thực thi:** Bazel phụ thuộc vào JVM daemon và hệ thống sandbox chuyên biệt. Fish chạy dưới dạng binary Rust độc lập, khởi động nhanh và tiêu thụ tài nguyên tối giản (~24 MB RAM so với hơn 650 MB của Bazel).

### Fish và Buck2
* **Quy trình làm việc:** Buck2 là hệ thống build hiệu năng cao sử dụng Starlark rules và file watcher ngoài. Fish hướng tới trải nghiệm đóng gói sẵn (out-of-the-box) với VFS trong bộ nhớ và GNU jobserver pool tích hợp mà không bắt buộc lập trình viên phải bảo trì hệ thống build phức tạp.

---

## Nghiên Cứu Điển Hình Thực Nghiệm: Bazel vs Fish trên `bazelbuild/examples`

> ⚠️ **Thông Báo Miễn Trừ Trách Nhiệm — Chỉ Mang Tính Tham Khảo:**
> Các số liệu đo đạc thực nghiệm dưới đây được ghi nhận trên một máy tính cá nhân chạy Windows x86_64 (4 nhân CPU, ~3.8 GB RAM) với kho dự án mẫu chính thức [`bazelbuild/examples`](https://github.com/bazelbuild/examples) của Google tại commit `3c479f4`.
> Bảng số liệu này **hoàn toàn mang tính chất tham khảo minh họa kỹ thuật**. Kết quả thực tế trong môi trường doanh nghiệp sẽ thay đổi tùy thuộc vào cấu hình phần cứng, tốc độ ổ đĩa SSD, băng thông mạng tải toolchain từ xa và độ ấm của cache. Bazel mang lại tính bảo đảm cách ly hermetic cấp trình biên dịch đòi hỏi chi phí khởi tạo lớn ban đầu, trong khi Fish ưu tiên trải nghiệm lập trình viên mượt mà (Zero-Config) và tốc độ thực thi bản địa tức thì.

### Thiết Lập Thử Nghiệm

Đo đạc trên cả 3 giai đoạn của dự án Go tutorial (`stage1`, `stage2`, `stage3`) trong `bazelbuild/examples`:
- **Quy trình xóa sạch bộ nhớ đệm (Clean Cache):**
  - **Bazel:** Chạy lệnh `bazel clean --expunge` để xóa sạch toàn bộ cache output, sandbox và tắt hoàn toàn tiến trình JVM nền.
  - **Fish:** Xóa sạch 100% thư mục `.fish/cache` và các thư mục `build/` cục bộ.
- **Phạm vi tác vụ:** Chỉ đo tác vụ biên dịch sinh file nhị phân thực thi (`go_binary` bên Bazel và `go build` với `run_tests = false` bên Fish).

### Bảng Kết Quả Thực Nghiệm

| Module Thử Nghiệm | Tên Mục Tiêu (Target) | Bazel 7.4.0 (Cold Build) | Bazel 7.4.0 (Warm Cached) | Fish 0.6.0 (Cold Build) | Fish 0.6.0 (Warm Cached) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Go Tutorial Stage 1** | `hello` | 165.53s | 23.55s | **1.08s** | **0.00092s (0.9ms)** |
| **Go Tutorial Stage 2** | `print_fortune` | 145.89s | 23.40s | **1.69s** | **0.00095s (0.9ms)** |
| **Go Tutorial Stage 3** | `fortune_test` | 149.68s | 23.70s | **0.99s** | **0.00088s (0.8ms)** |
| **Tổng Cộng 3 Dự Án** | **Toàn Bộ 3 Targets** | **461.10s (~7.7 phút)** | **~70.65s** | **3.76s** | **0.00275s (2.7ms)** |

### Phân Tích Kỹ Thuật

1. **Chênh lệch thời gian Cold Build (461.10s so với 3.76s):**
   - **Bazel:** Phải khởi động máy ảo Java (JVM), tải phiên bản Bazel 7.4, kéo bộ quy tắc `rules_go`, phân tích 101 packages và hơn 10.800 targets, biên dịch công cụ `builder.exe` và thư viện chuẩn Go trong các lớp sandbox cách ly.
   - **Fish:** Tận dụng trực tiếp chuỗi công cụ Go đã có sẵn trên máy với thời gian khởi động tức thì (< 15ms), bỏ qua các bước tải rườm rà và điều phối công việc trực tiếp vào hàng đợi work-stealing phi tập trung.

2. **Chênh lệch thời gian Warm Cache (~70.65s so với 0.00275s):**
   - **Bazel:** Ngay cả khi mã nguồn không đổi, Bazel vẫn cần kết nối lại với JVM daemon, phân tích lại đồ thị Starlark và đối chiếu hàm băm của hàng nghìn target.
   - **Fish:** Sử dụng cơ chế băm cây BLAKE3 để kiểm tra fingerprint tệp chỉ trong vài microsecond. Do tệp không đổi, Fish đạt **100% Cache Hits** và hoàn tất chỉ trong chưa đầy 3 milliseconds cho cả 3 dự án.
