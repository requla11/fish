# Bảng So Sánh: Fish và Các Hệ Thống Build Khác

Fish được thiết kế và xây dựng bằng Rust 2024 dành cho các monorepo đa ngôn ngữ hiện đại. Dưới đây là bảng so sánh kỹ thuật khách quan với Bazel, Turborepo và Buck2:

| Tính năng / Khía cạnh | Fish | Turborepo | Bazel | Buck2 |
| :--- | :--- | :--- | :--- | :--- |
| **Ngôn ngữ phát triển** | Rust 2024 | Go / Rust | Java / C++ | Rust |
| **Hỗ trợ ngôn ngữ** | Đa ngôn ngữ (11+ toolchains) | Tập trung JS / TS | Đa ngôn ngữ (Starlark) | Đa ngôn ngữ (Starlark) |
| **Mô hình cấu hình** | `fish.toml` tự động nhận diện | `turbo.json` | `WORKSPACE` + `BUILD.bazel` | `BUCK` files |
| **Độ phức tạp thiết lập** | Thấp / Tự động nhận diện | Thấp | Cao (cần cấu hình chi tiết) | Cao (cần cấu hình chi tiết) |
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
* **Môi trường thực thi:** Bazel phụ thuộc vào JVM daemon và hệ thống sandbox chuyên biệt. Fish chạy dưới dạng binary Rust độc lập, khởi động nhanh và tiêu thụ tài nguyên tối giản.

### Fish và Buck2
* **Quy trình làm việc:** Buck2 là hệ thống build hiệu năng cao sử dụng Starlark rules và file watcher ngoài. Fish hướng tới trải nghiệm đóng gói sẵn (out-of-the-box) với VFS trong bộ nhớ và GNU jobserver pool tích hợp mà không bắt buộc lập trình viên phải bảo trì hệ thống build phức tạp.
