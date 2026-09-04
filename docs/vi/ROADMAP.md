# Lộ trình Phát triển Fish (Roadmap)

> 🌐 **Bản dịch & Đóng góp:** Bạn muốn dịch hoặc hoàn thiện tài liệu này bằng ngôn ngữ của mình? Xem [Hướng dẫn Dịch thuật](TRANSLATION.md).

Tài liệu này trình bày lộ trình phát triển chiến lược của dự án Fish, từ các cột mốc đã hoàn thành đến các mục tiêu ngắn hạn, trung hạn, tầm nhìn dài hạn và các hướng nghiên cứu đột phá.

---

## 🎯 Tầm nhìn chiến lược

Fish hướng tới trở thành hệ thống điều phối build đa ngôn ngữ (polyglot) hiệu năng cao nhất, bền bỉ nhất và thân thiện nhất với lập trình viên cho các monorepo đa ngôn ngữ và môi trường phân tán, được vận hành bởi **lõi Rust (28 crates, Rust 2024, MSRV 1.88+) với 11 backend đa ngôn ngữ**. Các tầng phụ trợ Go/Python và hợp đồng `proto/` là các bản thảo hướng tới tương lai (xem `ARCHITECTURE.md`).

Các mục tiêu cốt lõi được tối ưu hóa theo thứ tự ưu tiên:

1. **Thời gian build thực tế (Wall-clock time)** — chỉ số trực tiếp nhất mà lập trình viên cảm nhận được.
2. **Hiệu suất Cache** — tỷ lệ cache hit, khả năng tái sử dụng artifact giữa các máy và khu vực.
3. **Độ tin cậy** — mọi byte dữ liệu trong cache đều khớp chính xác với input đầu vào.
4. **Tính trung thực của công cụ** — không tạo chẩn đoán giả, không báo thành công ảo.

---

## 🚀 Cột mốc Hiện tại (v0.2.x) — Đã Hoàn Thành

### Giai đoạn 1: Lõi Thực Thi & Đa Ngôn Ngữ
- [x] **Kiến trúc Lõi Rust**: Workspace Rust đơn ngữ (28 crates, resolver = "2", MSRV 1.88+) - không phụ thuộc `prost`/`tonic`; các tính năng phân tán sử dụng HTTP/JSON trực tiếp.
- [x] **11 Backend Ngôn ngữ**: Rust, Go, TypeScript/Node.js, Python, C/C++, Docker, Java, .NET, Swift, Dart, Zig.
- [x] **Bản thảo Hợp đồng Protobuf**: `proto/fish/v1/build.proto`, `ai.proto`, và `coordinator.proto` dưới dạng thiết kế sơ bộ.
- [x] **Lưu trữ CAS Blake3 & Thuật toán Nén ZSTD**: Hệ thống Content-Addressable Storage với thuật toán dọn dẹp hai pha.
- [x] **Điều phối GNU Jobserver**: Phân phối token luồng toàn cục giữa các compiler và đóng gói tài nguyên linh hoạt.
- [x] **Tự động sinh cấu hình CI/CD**: Hỗ trợ GitHub Actions, GitLab CI, CircleCI, Bitbucket.
- [x] **Hệ thống Tài liệu 5 Ngôn ngữ**: Triển khai trực tiếp trên GitHub Pages (Tiếng Anh, Tiếng Việt, Tiếng Trung Giản thể, Tiếng Trung Phồn thể, Tiếng Nhật).

---

## ⚡ Mục tiêu Ngắn hạn (v0.3.x) — Đã Hoàn Thành: Trải nghiệm Lập trình viên & Giao thức

### 1. Tích hợp IDE & Trình soạn thảo
- [x] **Extension VS Code**: Trực quan hóa đồ thị DAG tương tác, chạy tác vụ 1-click và chẩn đoán lỗi trực tiếp.
- [x] **Plugin Suite cho JetBrains**: Tích hợp cho CLion, IntelliJ IDEA và Rider với ToolWindow DAG và hỗ trợ LSP.
- [x] **Cầu nối LSP (Language Server Protocol)**: Chẩn đoán trực tiếp workspace và tự động hoàn thiện cú pháp `fish.toml`.

### 2. Giao thức IPC & Dịch vụ Tốc độ cao
- [x] **Luồng IPC Daemon**: Giao tiếp JSON-RPC 2.0 qua Unix domain socket / TCP fallback với độ trễ dưới 1 mili-giây.
- [x] **Giao thức gRPC REAPI**: Client REAPI v2 hoàn chỉnh cho các cụm worker phân tán (`fish-remote-cache/src/reapi.rs`).
- [x] **Theo dõi tệp bằng eBPF**: Phân tích hermeticity và phát hiện phụ thuộc động ở cấp nhân Linux (`fish-sandbox/src/ebpf.rs`).

### 3. Công cụ Chẩn đoán Thông minh & TUI
- [x] **Trợ lý Doctor AI tương tác**: Chẩn đoán chủ động và tự động sửa cấu hình môi trường (`fish doctor --fix`).
- [x] **Nâng cấp giao diện TUI**: Biểu đồ CPU/RAM sparkline thời gian thực và biểu đồ tiến trình dạng thác nước (waterfall).

---

## 🌟 Mục tiêu Trung hạn (v0.4.x - v0.5.x) — Trọng tâm: Hạ tầng Đám mây, AI & Quản lý Chi phí

### 1. Hạ tầng Phân tán Chuẩn Cloud-Native
- [x] **Kubernetes Operator (Go)**: Custom Resource Definitions (CRDs) để tự động co giãn worker pods (`go/pkg/k8s`).
- [x] **Tối ưu hóa Spot Instance**: Di chuyển và thử lại tác vụ an toàn khi nút đám mây bị thu hồi (`fish-scheduler/src/preemption.rs`).
- [x] **Sao chép Cache Đa Vùng**: Đồng bộ artifact CAS peer-to-peer với cache L2 phân tán theo địa lý (`fish-remote-cache/src/replication.rs`).

### 2. Tối ưu hóa Dự đoán & Machine Learning
- [x] **Dự đoán Thời gian Build**: Ước tính thời lượng tác vụ dựa trên EMA và lịch sử đo lường (`py/fish_optimizer/build_time_predictor.py`).
- [x] **Cách ly Flaky Test Tự động**: Phát hiện thống kê và cô lập các bài kiểm thử không tất định (`py/fish_recommender/flaky_quarantine.py`).
- [x] **Biên dịch đón đầu (Speculative Pre-Warming)**: Dự đoán các gói có khả năng bị sửa đổi và biên dịch trước ngầm (`fish-cli` + `py/fish_recommender/speculative_prewarmer.py`).

### 3. Đo lường Đo từ xa, Khả năng Quan sát & Hợp tác
- [x] **Tích hợp OpenTelemetry**: Truy vết phân tán OTLP xuyên suốt mọi bước build và node mạng (`fish-analytics/src/otel.rs`).
- [x] **Web Analytics Dashboard**: Server HTTP nội bộ cung cấp số liệu tốc độ build, hiệu suất cache và flamegraph (`fish-dashboard`).
- [x] **Ước tính Chi phí Đám mây (Cloud Cost Calculator)**: Tính toán chi phí thực tế và tiềm năng tiết kiệm trên AWS/GCP/Azure (`fish cost-estimate`).
- [x] **Hợp nhất Trace Phân tán**: Gộp các span từ toàn bộ worker vào một trace hoàn chỉnh (`fish-analytics/src/trace_merge.rs`).
- [x] **Cảnh báo Hồi quy Build**: Tự động phát hiện suy giảm tốc độ build giữa nhánh phát triển và PR.

### 4. Hệ sinh thái Plugin
- [x] **Plugin Engine trên WebAssembly**: Thực thi plugin WASI cô lập với bộ thông dịch `wasmi` (`fish-plugin/src/wasm.rs`).
- [x] **Plugin Marketplace Registry**: Khám phá và cài đặt plugin có chữ ký số Ed25519 (`fish plugin search|install|uninstall|publish`).
- [x] **Kiểm định Quyền Hạn Plugin (Capability Auditor)**: Phân tích tĩnh manifest để cảnh báo các quyền truy cập tệp/mạng quá mức.

### 5. Kỹ thuật Hiệu năng (Performance Engineering)
- [x] **Bộ Benchmark Đối chiếu**: Đo kiểm Fish so với Ninja và Bazel trên mô hình đồ thị đa ngôn ngữ (`crates/fish-scheduler/benches/peer_comparison.rs`).
- [x] **Ngân sách Điều phối (Overhead Budget)**: Giới hạn < 100µs cho mỗi quyết định phân phối tác vụ.
- [x] **Đọc CAS Zero-Copy**: Truy xuất artifact nóng qua `memmap2` không cần copy bộ nhớ (`fish-cas/src/mmap.rs`).
- [x] **Backend I/O Bất đồng bộ io_uring**: Tăng tốc đọc ghi I/O dung lượng lớn trên Linux (`fish-cas/src/uring.rs`, `fish-cache/src/uring.rs`).

---

## 🧭 v0.6.x — Trọng tâm: Độ tin cậy, Tính Hermetic & Chuỗi Cung Ứng

### 1. Quản lý Toolchain Thực Tế
- [x] **Tự động Tải Toolchain Hermetic**: Tải và xác minh checksum SHA-256 cho các toolchain Zig/Go/Node/CMake (`fish-core/src/toolchain_downloader.rs`).
- [x] **File Khóa Toolchain (Toolchain Lock File)**: Lưu trữ chính xác phiên bản toolchain trong `fish.lock`.
- [x] **Bảo đảm Chế độ Offline**: Mọi lệnh hoạt động tất định khi không có mạng với cờ `--offline`.

### 2. Khả năng Tái Lập (Build Reproducibility)
- [x] **Phát lại Dấu vết (Trace Replay)**: Lưu trữ và phát lại toàn bộ tiến trình thực thi để chứng minh tính hermetic (`fish-executor/src/trace_replay.rs`).
- [x] **Chứng nhận Tái lập Từng Bit (Bit-for-Bit Certification)**: So sánh hash BLAKE3 output directory để kiểm tra tính tất định.
- [x] **Phát hiện Trôi Môi trường (Environment Drift Detector)**: Cảnh báo khi cấu hình compiler/OS có sự biến đổi so với lần build thành công trước.

### 3. Tăng cường Bảo mật
- [x] **Hồ sơ Chính sách Sandbox**: Cấu hình các cấp độ bảo vệ (`strict`, `default`, `trusted`) cho môi trường thực thi.
- [x] **Cổng Xác thực Chữ ký Artifact**: Từ chối các artifact CAS từ xa không có chữ ký Ed25519 tin cậy.
- [x] **Kiểm toán Lỗ hổng Phụ thuộc**: Tích hợp quét lỗ hổng trực tiếp từ nguồn cấp dữ liệu RustSec/OSV (`fish-security/src/osv.rs`).

---

## 🤖 v0.7.x — Trọng tâm: Xây Dựng Bản Build Tự Nhiên Với AI

- [x] **Tự Động Sửa Lỗi Biên Dịch**: `fish fix` phân tích lỗi `cargo check` và áp dụng patch an toàn với diff rõ ràng.
- [x] **Truy Vấn Build Bằng Ngôn Ngữ Tự Nhiên**: `fish why --ask "tại sao core bị rebuild?"` dựa trên dữ liệu trace và fingerprint thực tế.
- [x] **Điều tiết Tài nguyên Thích ứng**: Dự đoán mức tiêu thụ RAM theo phân vị P90 để định cỡ job pool linh hoạt.
- [x] **Lựa Chọn Bài Test Thông Minh (Test Selection)**: Bỏ qua các test không bị ảnh hưởng bởi file thay đổi dựa trên đồ thị tác động.
- [x] **Lưu Trữ Chuỗi Thời Gian Build**: Ghi nhận metric cục bộ bằng SQLite WAL (`fish-analytics/src/time_series.rs`).

---

## 🏛️ Tầm nhìn Dài hạn (v1.0+) — Trọng tâm: Doanh nghiệp & Zero-Trust

- [x] **Cách ly Phần cứng MicroVM**: Chạy build hermetic bên trong Firecracker / Cloud-Hypervisor microVMs.
- [ ] **Định danh Doanh nghiệp (SSO / OIDC)**: Quản lý quyền truy cập RBAC và nhật ký kiểm toán cho các target nhạy cảm.
- [ ] **Nguồn gốc Chuỗi Cung ứng Mật mã (SLSA Level 3)**: Sinh chứng chỉ in-toto attestation chống giả mạo.
- [x] **Bộ Điều Phối Sẵn Sàng Cao (HA Coordinator)**: Quản lý cụm worker với thuật toán đồng thuận Raft trong Go.
- [x] **Cách ly Cache Đa Thuê bao (Multi-Tenant)**: Phân chia namespace CAS với hạn ngạch theo từng team.
- [x] **AST Sub-Tree Caching Đa Ngôn Ngữ**: Biên dịch tăng dần ở cấp độ từng hàm / khối mã.
- [x] **Mạng Lưới P2P Mesh Toàn Cầu**: Chia sẻ artifact CAS phân tán lấy cảm hứng từ BitTorrent.
- [x] **Bộ Tối Ưu Hóa Liên Tục**: Tự động tinh chỉnh cờ và cấu hình build để đạt tốc độ tối đa.

---

## 🚀 Hướng Nghiên Cứu Đột Phá (v2.0 Moonshots)

- [ ] **Compiler Query Hooks**: Tích hợp sâu vào rustc/tsc/clang để cung cấp đơn vị biên dịch trực tiếp cho bộ điều phối.
- [x] **Builds Tự Phục Hồi (Self-Healing Builds)**: Tự động phân tích nguyên nhân lỗi và chuẩn bị PR khắc phục an toàn.
- [x] **Điều Phối Nhận Biết Carbon (Carbon-Aware Scheduling)**: Lên lịch tác vụ linh hoạt vào khung giờ lưới điện phát thải carbon thấp.
- [ ] **Liên Minh Build Mesh Toàn Cầu**: Chia sẻ ẩn danh các khối CAS phổ biến giữa các tổ chức.
- [ ] **Khởi Tạo Cấu Hình Build Bằng Ngôn Ngữ Tự Nhiên**: Tự động sinh `fish.yaml` chuẩn từ mô tả bằng lời.

---

## 📅 Ước tính Tiến độ

| Phiên bản | Trọng tâm | Mốc thời gian | Trạng thái |
| :--- | :--- | :--- | :--- |
| **v0.2.x** | Lõi Rust, 11 Backends, CAS, Tài liệu 5 ngôn ngữ | Q3 2026 | ✅ Đã hoàn thành |
| **v0.3.x** | Plugin IDE, Cầu nối IPC, eBPF Tracing, LSP | Q3 2026 | ✅ Đã hoàn thành |
| **v0.4.x - v0.5.x** | K8s Operator, Predictive ML, OpenTelemetry, Cost Calculator | Q1 - Q2 2027 | 🟡 Đang phát triển |
| **v0.6.x** | Hermeticity, Toolchain Provisioning, Supply Chain Security | Q2 - Q3 2027 | ⚪ Đã lên kế hoạch |
| **v0.7.x** | AI-Native Builds, Learned Resources, Test Selection | Q3 - Q4 2027 | ⚪ Đã lên kế hoạch |
| **v1.0** | MicroVM Sandboxing, Enterprise SSO, P2P Mesh, SLSA L3 | Q1 2028+ | ⚪ Tầm nhìn |
| **v2.0** | Compiler Query Hooks, Self-Healing, Carbon-Aware, Federation | Tương lai xa | 🔮 Nghiên cứu |
