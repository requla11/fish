# Các Thuật Toán Nâng Cao & Cải Tiến Kiến Trúc Của Fish

> 🌐 **Điều hướng ngôn ngữ / Language Navigation:**
> [English](../proprietary-tech.md) | [Tiếng Việt](proprietary-tech.md) | [日本語](../ja/proprietary-tech.md) | [简体中文](../zh-hans/proprietary-tech.md) | [繁體中文](../zh-hant/proprietary-tech.md)

---

## ⚡ Tổng Quan: Các Cải Tiến Cốt Lõi Trong Fish

Fish tích hợp bốn thuật toán chuyên biệt được thiết kế để giải quyết bài toán quy mô, hủy cache (invalidation) và độ trễ tăng dần trong monorepo đa ngôn ngữ:

```
+-------------------------------------------------------------------------------+
|                      CÁC CẢI TIẾN THUẬT TOÁN CỐT LÕI CỦA FISH                 |
+-------------------------------------------------------------------------------+
|  1. Poly-ABI Semantic HyperGraph (PASH)                                       |
|     --> Trích xuất symbol ranh giới interface & ngăn chặn cascade hủy cache   |
|                                                                               |
|  2. Iso-Semantic Morphic Fingerprinting (IS-MFP)                              |
|     --> Chuẩn hóa CAS Dual-Key loại bỏ tình trạng cache miss giữa Local & CI  |
|                                                                               |
|  3. Speculative Wavelet Pre-Execution (SWPE)                                  |
|     --> Phân loại mức năng lượng thao tác gõ phím & pre-warm đồ thị tác vụ    |
|                                                                               |
|  4. Virtual Binary Dispatch Table (CAS-VLink)                                 |
|     --> Bảng overlay dispatch nhị phân trên RAM cho vòng lặp lặp lại nhanh    |
+-------------------------------------------------------------------------------+
```

---

## 🔬 1. Poly-ABI Semantic HyperGraph (PASH)
* **Vị trí**: `crates/fish-graph`, `crates/fish-core`
* **Vấn đề**: Các hệ thống build truyền thống thường vô hiệu hóa toàn bộ target phụ thuộc khi bất kỳ file nguồn nào thay đổi, ngay cả khi interface công khai (API/signature) không hề thay đổi.
* **Cơ chế**:
  * Quét các chữ ký interface công khai được export trên toàn bộ 11 backend hỗ trợ (Rust, C/C++, Go, TS/JS, Python, Java, .NET, Swift, Dart, Zig, Docker).
  * Tính toán giá trị băm bất biến `Symbolic Boundary Hash (SBH)`.
  * Khi các chi tiết cài đặt nội bộ thay đổi nhưng `SBH` giữ nguyên, PASH sẽ ngắt chuỗi hủy cache lan truyền, tiết kiệm đáng kể thời gian build lại.

---

## 🧬 2. Iso-Semantic Morphic Fingerprinting (IS-MFP)
* **Vị trí**: `crates/fish-cache`, `crates/fish-cas`
* **Vấn đề**: Sự khác biệt về đường dẫn thư mục, định dạng và biến môi trường khiến tỷ lệ cache hit thường về 0% khi chuyển đổi giữa máy phát triển cục bộ và CI runner.
* **Cơ chế**:
  * Triển khai **Kiến trúc Băm Khóa Kép (Dual-Key)** gồm `ExactKey` và `MorphicKey`.
  * Chuẩn hóa đường dẫn tương đối (chuyển dấu `\` của Windows sang `/`) và lọc nhiễu môi trường biến động.
  * Tự động fallback sang khớp morphic khi không trúng exact hit, tối đa hóa khả năng tái sử dụng cache.

---

## 🌊 3. Speculative Wavelet Pre-Execution (SWPE)
* **Vị trí**: `crates/fish-incremental`
* **Vấn đề**: Các hệ thống build thụ động phải đợi thao tác lưu file hoặc lệnh build từ terminal, gây tích tụ độ trễ cho lập trình viên.
* **Cơ chế**:
  * Phân loại các thay đổi từ trình soạn thảo thành các mức năng lượng (`TrivialWhitespace`, `CommentOnly`, `InternalStatement`, `GlobalInterface`).
  * Chuẩn bị sẵn sàng trạng thái phụ thuộc của tác vụ và buffer artifact trong bộ nhớ ngầm trước khi thực hiện lệnh build hoàn chỉnh.

---

## ⚡ 4. Virtual Binary Dispatch Table (CAS-VLink)
* **Vị trí**: `crates/fish-executor`
* **Vấn đề**: Quá trình liên kết (link) toàn bộ file nhị phân tiêu tốn thời gian đáng kể trong các vòng lặp phát triển nhỏ.
* **Cơ chế**:
  * Duy trì một bảng `VirtualBinaryDispatchTable` trong bộ nhớ ánh xạ địa chỉ symbol và khối bytecode.
  * Sinh ra cấu trúc runtime binary overlay (`VLINK_DISPATCH_HEADER_V1`) hỗ trợ thay thế symbol nhanh chóng trong chu kỳ kiểm thử tăng dần.
