# Vấn đề giải quyết
* Xem video an toàn: Upload các video có nguy cơ bảo mật cao để xem trực tiếp trên web, lưu trữ và xem lại sau.
* Tải video gián tiếp: Tải trực tiếp video từ link và lưu vào Object Storage (Cloudflare R2) mà không cần tải qua thiết bị cá nhân, tránh chiếm dụng băng thông và dung lượng cục bộ.

---

# Tech Stack (gọn)
* Backend: Rust
* Frontend: TypeScript (React)
* Storage: Cloudflare R2
* Xác thực: Google Auth (GG Auth)
* ...  (chi tiết xem ở [ChiTiet.md](./ChiTiet.md))

---

# Yêu cầu hệ thống
Trong quá trình thử nghiệm, server với cấu hình 512MB RAM (Render free tier) sẽ bị crash

Giải pháp (chọn 1 trong 2):
1. Cắt bỏ đoạn mã nguồn liên quan đến trích xuất ảnh thu nhỏ (thumbnail) bằng `ffmpeg`, nếu không biết làm, bạn có thể nhờ trợ lý AI xử lý giùm bạn việc này
2. Sử dụng cấu hình máy chủ có dung lượng 1GB RAM trở lên.

---

# Hướng dẫn triển khai
Bạn có thể nhờ trợ lý AI hỗ trợ cài đặt/triển khai từ mã nguồn hoặc tham khảo tài liệu hướng dẫn từng bước chi tiết tại:
* [ChiTiet.md](./ChiTiet.md)

