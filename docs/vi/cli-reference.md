# Danh mục Tham khảo Lệnh Fish CLI

> 🌐 **Bản dịch & Đóng góp:** Bạn muốn dịch hoặc cải thiện tài liệu này bằng ngôn ngữ của mình? Xem [Hướng dẫn Dịch thuật](TRANSLATION.md).

Tài liệu tham khảo toàn diện về tất cả các lệnh, cờ (flags), và tùy chọn cấu hình của giao diện dòng lệnh Fish.

---

## 🧭 Cú pháp Chung & Tùy chọn Toàn cục

```bash
fish [OPTIONS] <COMMAND>
```

### Các Cờ Toàn Cục (Global Flags)

| Cờ (Flag) | Mô tả | Mặc định |
|---|---|---|
| `--experimental` | Bật các tính năng thử nghiệm của Fish. | `false` |
| `--offline` | Vô hiệu hóa truy cập mạng, từ chối thực hiện các tác vụ từ xa. | `false` |
| `-v, --verbose` | Bật đầu ra chi tiết và nhật ký chẩn đoán. | `false` |
| `-j, --jobs <N>` | Số lượng luồng worker thực thi song song tối đa. | Số lõi CPU |
| `--no-cache` | Vô hiệu hóa việc đọc/ghi cache (cả cục bộ lẫn từ xa). | `false` |
| `--cache-dir <PATH>` | Đường dẫn thư mục cache cục bộ (mặc định: `~/.fish/cache`). | Mặc định hệ thống |

---

## 🛠️ Danh sách Tất cả Lệnh Subcommand

---

### `fish init`
Khởi tạo cấu hình Fish (`fish.toml`) và quét workspace đa ngôn ngữ.

```bash
fish init [OPTIONS]
```
- `-p, --path <PATH>`: Thư mục cần khởi tạo.
- `-f, --force`: Ghi đè cấu hình hiện có nếu đã tồn tại.
- `--describe <DESC>`: Mô tả dự án bằng ngôn ngữ tự nhiên để AI cấu hình.

---

### `fish new`
Tạo một dự án hoặc package mới từ các mẫu (template) có sẵn.

```bash
fish new <NAME> [OPTIONS]
```
- `-t, --template <TEMPLATE>`: Tên mẫu dự án (ví dụ: `rust`, `ts`, `go`, `polyglot`).
- `-p, --path <PATH>`: Thư mục đích.

---

### `fish build`
Thực thi các tác vụ biên dịch và build cho workspace.

```bash
fish build [OPTIONS] [PATH]
```
- `-j, --jobs <N>`: Giới hạn số worker song song.
- `-v, --verbose`: In chi tiết các bước thực thi.
- `--no-cache`: Bỏ qua cache.
- `--sandbox`: Chạy tác vụ bên trong sandbox an toàn.
- `--apple`: Chạy qua sandbox kín `apple`.
- `--profile <FILE>`: Xuất Chrome trace JSON phân tích hiệu năng.
- `--tui`: Bật giao diện Terminal UI tương tác.
- `--remote-cache <URL>`: Địa chỉ máy chủ Remote Cache (HTTP hoặc gRPC REAPI).
- `--remote-workers <URL>`: Cụm worker từ xa để điều phối DTE.
- `--ram-limit <PCT>`: Tự động giảm mức độ đồng thời khi RAM khả dụng dưới tỷ lệ này.
- `--semantic`: Bật cache ngữ nghĩa cấp độ AST.
- `--reflink`: Sử dụng copy-on-write (reflink) khi khôi phục artifact từ CAS.
- `--critical-path`: Ưu tiên xếp lịch cho các nhánh trên đường găng (critical path).
- `--explain`: In lý do chi tiết tại sao các tác vụ bị build lại.
- `--otel-endpoint <URL>`: Xuất OpenTelemetry trace tới OTLP collector.
- `--no-infer-deps`: Tắt tự động suy luận phụ thuộc chéo ngôn ngữ.

---

### `fish check`
Kiểm tra cú pháp, phân tích tĩnh và type-check mà không liên kết mã máy.

```bash
fish check [OPTIONS] [PATH]
```
- Hỗ trợ các cờ tương tự `fish build` (`--jobs`, `--no-cache`, `--sandbox`, `--explain`, v.v.).

---

### `fish test`
Chạy toàn bộ các bộ kiểm thử trong workspace.

```bash
fish test [OPTIONS] [PATH]
```
- `--quarantine-flaky`: Tự động cách ly các bài test không tất định (flaky tests).
- `--test-threads <N>`: Số luồng chạy test đồng thời.

---

### `fish clean`
Dọn dẹp các tệp build tạm thời và giải phóng bộ nhớ cache.

```bash
fish clean [OPTIONS]
```
- `--all`: Dọn sạch cả cache cục bộ CAS và L1/L2.
- `--dry-run`: Liệt kê các tệp sẽ bị xóa mà không thực hiện xóa.

---

### `fish run`
Biên dịch và chạy một target thực thi cụ thể.

```bash
fish run -p <PACKAGE> [--bin <BINARY>] [-- <ARGS>...]
```

---

### `fish graph`
Xuất và trực quan hóa đồ thị phụ thuộc (DAG) của workspace.

```bash
fish graph [OPTIONS]
```
- `--format <FORMAT>`: Định dạng xuất (`dot`, `json`, `mermaid`, `svg`).
- `--output <FILE>`: Ghi đồ thị ra tệp.

---

### `fish watch`
Theo dõi các thay đổi tệp trên đĩa và tự động kích hoạt build lại tăng dần.

```bash
fish watch [OPTIONS]
```
- `--debounce <MS>`: Thời gian chờ gom nhóm sự kiện thay đổi (mặc định: 200ms).

---

### `fish query`
Thực thi các biểu thức đại số truy vấn đồ thị phụ thuộc.

```bash
fish query "<EXPRESSION>"
```
- `deps(//pkg)`: Phụ thuộc xuôi của package.
- `rdeps(//pkg)`: Phụ thuộc ngược của package.
- `allpaths(//a, //b)`: Tất cả các đường đi giữa hai target.
- `somepath(//a, //b)`: Đường đi ngắn nhất giữa hai target.

---

### `fish doctor`
Kiểm tra sức khỏe môi trường, công cụ toolchain và cấu hình Fish.

```bash
fish doctor [OPTIONS]
```
- `--fix`: Tự động sửa chữa các vấn đề về quyền, tệp tạm và cấu hình `fish.toml`.
- `--ai`: Sử dụng mô hình AI chẩn đoán chuyên sâu và đề xuất giải pháp.

---

### `fish why`
Giải thích nguyên nhân tại sao một package hoặc target bị rebuild.

```bash
fish why <TARGET> [OPTIONS]
```
- `--ask "<QUESTION>"`: Đặt câu hỏi bằng ngôn ngữ tự nhiên về nguyên nhân rebuild.

---

### `fish fix`
Tự động áp dụng các sửa đổi an toàn dựa trên cảnh báo và lỗi của trình biên dịch.

```bash
fish fix [OPTIONS]
```
- `--diff`: Hiển thị bản vá dưới dạng Git unified diff trước khi áp dụng.
- `--apply`: Áp dụng trực tiếp bản sửa lỗi vào mã nguồn.

---

### `fish affected`
Xác định danh sách các package bị ảnh hưởng bởi các tệp sửa đổi so với Git commit/nhánh gốc.

```bash
fish affected --base <REF> [--head <REF>]
```

---

### `fish cache`
Quản lý, kiểm toán dung lượng và tối ưu hóa hệ thống Content-Addressable Storage (CAS).

```bash
fish cache <SUBCOMMAND>
```
- `prune`: Dọn dẹp các khối dữ liệu cũ dựa trên LRU và ngưỡng dung lượng.
- `stats`: Hiển thị tỷ lệ cache hit/miss và dung lượng chiếm dụng.
- `verify`: Kiểm tra tính toàn vẹn SHA-256/BLAKE3 của tất cả artifact trong CAS.

---

### `fish cost-estimate`
Tính toán chi phí tài nguyên đám mây và ước tính khoản tiết kiệm chi phí build trên AWS, GCP, Azure.

```bash
fish cost-estimate [OPTIONS]
```
- `--json`: Xuất dữ liệu dưới định dạng JSON cho hệ thống CI/CD.

---

### `fish ui`
Khởi chạy giao diện Web Dashboard phân tích hiệu năng và biểu đồ DAG.

```bash
fish ui [--port <PORT>] [--open]
```

---

### `fish pash`
Hiển thị và kiểm tra thuật toán băm nhận biết ngữ nghĩa (Path-Aware Semantic Hashing).

```bash
fish pash <TARGET>
```

---

### `fish qpc`
Truy vấn bộ nhớ đệm pipeline (Query Pipeline Cache) để kiểm tra các phần tử biên dịch tăng dần.

```bash
fish qpc <TARGET>
```

---

### `fish attest` & `fish verify`
Ký số mật mã (Ed25519) và xác minh chứng chỉ nguồn gốc SLSA / in-toto cho artifact.

```bash
fish attest --out <ATTESTATION_FILE>
fish verify --attestation <ATTESTATION_FILE>
```

---

### `fish lsp` & `fish daemon`
Chạy máy chủ LSP cho IDE hoặc tiến trình nền daemon để tối ưu hóa IPC.

```bash
fish lsp
fish daemon [--socket <PATH>]
```
