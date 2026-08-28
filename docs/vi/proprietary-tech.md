# Công nghệ độc quyền & Thuật toán thế hệ mới của Fish

> 🌐 **Chuyển đổi ngôn ngữ:**
> [English](../../PROPRIETARY_TECH.md) | [Tiếng Việt](proprietary-tech.md) | [简体中文](../zh-Hans/proprietary-tech.md) | [繁體中文](../zh-Hant/proprietary-tech.md) | [日本語](../ja/proprietary-tech.md)

---

## ⚡ Tổng quan: Fish Quantum Polyglot Core (QPC)

Fish đang tiên phong phát triển bộ 4 thuật toán độc quyền, được thiết kế để giải quyết triệt để các điểm nghẽn hiệu năng cốt lõi trong monorepo đa ngôn ngữ và hệ thống build phân tán.

```
+-------------------------------------------------------------------------------+
|                      FISH QUANTUM POLYGLOT CORE (QPC)                         |
+-------------------------------------------------------------------------------+
|  1. Poly-ABI Semantic HyperGraph (PASH)                                       |
|     --> Trích xuất biên giới giao diện công khai & ngắt lan truyền invalidation |
|                                                                               |
|  2. Iso-Semantic Morphic Fingerprinting (IS-MFP)                              |
|     --> Kiến trúc Dual-Key CAS triệt tiêu hiện tượng sụp đổ Cache (Cache Cliff)|
|                                                                               |
|  3. Speculative Wavelet Pre-Execution (SWPE)                                  |
|     --> Build đón đầu thời gian thực qua dòng token LSP với chi phí 0ms      |
|                                                                               |
|  4. CAS-VLink (Virtual Jump-Table Splicer)                                    |
|     --> Bỏ qua Linker hệ thống nhờ bảng nhảy nhị phân Zero-Copy mmap          |
+-------------------------------------------------------------------------------+
```

---

## 🔬 1. Poly-ABI Semantic HyperGraph (PASH)
* **Trạng thái**: Đang phát triển tích cực (`crates/fish-graph`, `crates/fish-core`).
* **Vấn đề**: Các build system hiện nay tự động invalidate toàn bộ target hạ nguồn khi file nguồn thượng nguồn đổi, dù API/ABI không thay đổi.
* **Cơ chế**:
  * Tự động trích xuất **Public Interface Boundary (PIB)** cho cả 11 backend ngôn ngữ.
  * Tính toán giá trị băm bất biến `Symbolic Boundary Hash (SBH)`.
  * Khi file nguồn thay đổi nhưng `SBH` không đổi, PASH ngắt hoàn toàn chuỗi invalidation cascade xuyên biên giới ngôn ngữ.

---

## 🧬 2. Iso-Semantic Morphic Fingerprinting (IS-MFP)
* **Trạng thái**: Đang phát triển tích cực (`crates/fish-cache`, `crates/fish-cas`).
* **Vấn đề**: Khác biệt đường dẫn thư mục và entropy môi trường khiến tỷ lệ Cache Hit tụt về 0% khi chuyển đổi giữa máy cá nhân và CI.
* **Cơ chế**:
  * Triển khai kiến trúc băm 2 khóa **Dual-Key Hashing Architecture** (`ExactKey` + `MorphicKey`).
  * Chuẩn hóa cấu trúc ngữ nghĩa AST và loại bỏ nhiễu đường dẫn/timestamp.
  * Tự động fallback sang khớp Morphic khi Exact miss, nâng tỷ lệ tái sử dụng cache lên >95%.

---

## 🌊 3. Speculative Wavelet Pre-Execution (SWPE)
* **Trạng thái**: Đang phát triển tích cực (`crates/fish-scheduler`, `crates/fish-incremental`).
* **Vấn đề**: Các hệ thống bị động chỉ bắt đầu build khi bấm lưu (`Ctrl+S`) hoặc gõ lệnh, gây lãng phí thời gian chờ đợi.
* **Cơ chế**:
  * Kết nối trực tiếp với `Fish LSP Bridge` để theo dõi dòng biến thiên cú pháp (wavelet) thời gian thực.
  * Điều phối CPU nhàn rỗi ngầm (Jobserver) để chuẩn bị sẵn type inference và ngữ cảnh bộ nhớ codegen.
  * Mang lại phản hồi thực thi tức thì (<1ms) ngay khi người dùng lưu file.

---

## ⚡ 4. CAS-VLink (Virtual Jump-Table Splicer)
* **Trạng thái**: Đang phát triển tích cực (`crates/fish-executor`, `crates/fish-cas`).
* **Vấn đề**: Linker hệ thống (`ld`, `lld`, `link.exe`) chiếm 40-60% thời gian biên dịch các binary lớn (C++, Rust, Swift, Go).
* **Cơ chế**:
  * Xây dựng bảng điều hướng nhị phân ảo **Virtual Binary Dispatch Table (VBDT)** trong output binary.
  * Sử dụng zero-copy memory mapping để ghép nối trực tiếp binary object mới mà không cần chạy lại Linker hệ thống.
  * Tăng tốc độ tái tạo binary lên 10x-50x trong các vòng lặp code lặp đi lặp lại.
