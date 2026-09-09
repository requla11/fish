# Ký & Xác Thực Artifact (Signing & Verification)

Fish có thể ký số mọi artifact khi đẩy lên remote cache và từ chối mọi tải xuống nếu chữ ký không hợp lệ. Trang này hướng dẫn chi tiết cách thiết lập toàn diện từ đầu đến cuối.

## Khái niệm

| Thuật ngữ | Ý nghĩa |
|---|---|
| **Seed** | 32 byte ngẫu nhiên (64 ký tự hex). Dùng để tạo cặp khóa ký Ed25519. Phải giữ bí mật. |
| **Khóa công khai (Public key)** | Được tạo từ seed thông qua lệnh `fish signing-key`. An toàn để chia sẻ; dùng để xác minh chữ ký. |
| **Cổng chữ ký (Signature gate)** | Middleware trên client remote-cache: ký khi tải lên (upload), xác thực khi tải xuống (download). |

## 1. Tạo signing seed

```powershell
# Windows (PowerShell, .NET crypto RNG)
$rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
$bytes = New-Object byte[] 32
$rng.GetBytes($bytes)
($bytes | ForEach-Object { $_.ToString("x2") }) -join ""
```

```bash
# macOS / Linux
openssl rand -hex 32
```

Hãy sao lưu seed cẩn thận trong trình quản lý mật khẩu. Mất seed đồng nghĩa với mất danh tính ký; lộ seed đồng nghĩa với việc kẻ xấu có thể giả mạo nguồn gốc build của bạn.

## 2. Xuất khóa công khai

```bash
export FISH_SIGNING_SEED=<chuỗi-64-ký-tự-hex>
fish signing-key
# -> 87362bc246e5fe912fa774cfa728cece02545fc3ef7abae394c65e30a2da9455
```

Lệnh này chỉ in khóa công khai — seed sẽ không bao giờ xuất hiện trong output.

## 3. Ký bản build (Phía Producer)

```bash
export FISH_SIGNING_SEED=<seed>
fish build            # các artifact tải lên sẽ được ký tự động
```

## 4. Xác thực artifact (Phía Consumer)

```bash
export FISH_SIGNING_SEED=<seed>                      # cùng danh tính, nếu cần ký lại khi đẩy
export FISH_TRUSTED_KEYS=87362bc2...9455             # hỗ trợ danh sách phân tách bằng dấu phẩy
export FISH_SIG_POLICY=refuse                        # mặc định; dùng "warn" để chỉ log cảnh báo thay vì dừng

fish build           # các bản tải xuống không vượt qua xác thực sẽ bị từ chối
```

Chính sách (Policies):

- `refuse` (mặc định): tải xuống bị thiếu chữ ký/chữ ký không hợp lệ hoặc được ký bởi khóa không tin cậy sẽ làm task thất bại.
- `warn`: chấp nhận nhưng in cảnh báo cho từng artifact vi phạm.

## Tích hợp CI/CD

Thiết lập `RELEASE_SIGNING_SEED` làm repository secret; `.github/workflows/release.yaml` sẽ dùng seed này để ký các báo cáo nguồn gốc SLSA (provenance) khi phát hành release. Các bên tiêu thụ có thể cấu hình khóa công khai trong biến môi trường `FISH_TRUSTED_KEYS` trên runner của họ.

## Thu hồi và đổi khóa (Key Rotation)

1. Tạo một seed mới.
2. Thêm khóa công khai **mới** vào `FISH_TRUSTED_KEYS` song song với khóa cũ.
3. Build lại và xuất bản lại các artifact.
4. Xóa khóa cũ khỏi danh sách tin cậy khi không còn consumer nào phụ thuộc vào nó.
