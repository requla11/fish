# Nhật Ký Thay Đổi & Lịch Sử Phiên Bản

Tất cả các thay đổi đáng chú ý của dự án Fish được ghi lại tại đây.

Định dạng dựa trên [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) và tuân thủ [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.6.0] - 2026-08-25

### Thêm Mới
- **Giao Thức Protobuf Liên Dịch Vụ**: Bộ mã hóa/giải mã nhị phân Google Protocol Buffers wire protocol trên cả 3 ngôn ngữ Rust, Go và Python mà không cần phụ thuộc binary ngoài.
- **Wasm Plugin Engine & Kiểm Tra An Toàn**: Cơ chế plugin WebAssembly cách ly an toàn, kiểm tra phân quyền năng lực (`fish plugin audit`) và xác thực chữ ký điện tử Ed25519.
- **Kho Lưu Trữ CAS với ZSTD**: Hashing BLAKE3 siêu tốc kết hợp nén Zstandard đa luồng cho hệ thống cache L1/L2 định danh nội dung.
- **11 Backend Đa Ngôn Ngữ**: Hỗ trợ zero-config cho Rust, Go, TypeScript/Node, Python, C/C++, Docker, Java, .NET, Swift, Dart và Zig.
- **Lập Lịch Song Song Thích Ứng & Work-Stealing**: Thuật toán Chase-Lev work-stealing với hàng đợi phi tập trung, ưu tiên chuỗi găng (critical-path) và kiểm soát áp lực bộ nhớ RAM.

### Cải Tiến
- Báo cáo chu trình phụ thuộc (cycle) với đường dẫn chi tiết thay vì lỗi chung chung.
- Phục hồi cache cục bộ tự động giải nén đầy đủ artifact khai báo về đĩa.

## [v0.5.0] - 2026-08-24

### Thêm Mới
- **Cổng Tài Liệu 5 Ngôn Ngữ**: Hệ thống tài liệu VitePress hoàn chỉnh hỗ trợ Tiếng Anh, Tiếng Việt, Trung Giản thể, Trung Phồn thể và Tiếng Nhật.
- **Điều Phối Viên Phân Tán (Go)**: Module coordinator cụm worker hiệu năng cao với theo dõi heartbeat và endpoint HTTP/Protobuf.
- **AI Phân Tích Lỗi & Sửa Đổi (Python)**: Cầu nối tiến trình phân tích lỗi trình biên dịch và dự đoán làm ấm bộ nhớ đệm.

## [v0.3.0] - 2026-08-21

### Thêm Mới
- **Phần Mở Rộng IDE**: Extension chính thức cho VS Code và máy chủ Language Server Protocol (`fish lsp`).
- **Bảng Điều Khiển TUI Tương Tác**: Theo dõi tiến độ build đa luồng, mức sử dụng CPU/RAM và biểu đồ thác nước (waterfall) theo thời gian thực.
- **Truy Vết eBPF**: Phát hiện phụ thuộc và truy cập tệp động ở tầng nhân Linux.

## [v0.2.0] - 2026-08-10

### Thêm Mới
- **Kiến Trúc Lõi Tam Động Cơ (Tri-Engine)**: Nhân điều phối viết bằng Rust 2024 kết hợp các dịch vụ phân tán Go và AI Python.
- **Động Cơ Cache Fingerprint**: Mã băm BLAKE3 tốc độ cao cho khóa cache tác vụ và phát hiện thay đổi.
- **Bộ Quản Lý GNU Jobserver**: Kiểm soát mức độ song song toàn cục tránh nghẽn tài nguyên.

## [v0.1.0] - 2026-08-01

### Thêm Mới
- Bản phát hành thử nghiệm đầu tiên của Fish với hỗ trợ Rust và TypeScript.
