# 🎥 Safe Stream - Hệ Thống Video Streaming Cá Nhân

Hệ thống truyền tải video (video streaming) cá nhân an toàn, tối ưu hiệu năng và chi phí. Dự án sử dụng kiến trúc phân tách với **Backend viết bằng Rust (Axum)** và **Frontend bằng React (TypeScript + Vite)**, kết hợp cùng **Cloudflare R2** để lưu trữ file với chi phí băng thông $0.

Ứng dụng hỗ trợ cơ chế xác thực bảo mật qua Google OAuth (chỉ cho phép các email được chỉ định truy cập) và tích hợp các tính năng nâng cao như trích xuất thumbnail bằng ffmpeg, tải video an toàn từ URL (chống tấn công SSRF).

---

## 🛠️ Công Nghệ Sử Dụng (Tech Stack)

### 1. Backend (Rust)
* **Framework:** [Axum](https://github.com/tokio-rs/axum) - Web framework hiệu năng cao, xây dựng trên nền tảng Tokio async.
* **Storage:** AWS SDK Rust (tương thích Cloudflare R2).
* **Multimedia:** Tích hợp `ffmpeg` để tự động tạo ảnh thu nhỏ (thumbnail) từ video tải lên.
* **Security:** Middleware CORS tùy chỉnh, kiểm tra token Google Auth, xác thực tài khoản nghiêm ngặt, cơ chế bảo vệ SSRF khi tải file từ URL.

### 2. Frontend (React)
* **Framework:** React 18, TypeScript, Vite.
* **Styling:** Vanilla CSS tối ưu dung lượng tải trang, thiết kế hiện đại, responsive.
* **Icons:** Lucide React.

### 3. Storage & Infrastructure
* **Object Storage:** [Cloudflare R2](https://www.cloudflare.com/developer-platform/r2/) (Hỗ trợ API tương thích S3, miễn phí 100% băng thông tải xuống).
* **Identity Provider:** Google Developer Console (OAuth 2.0).

---

## 📋 Yêu Cầu Hệ Thống (System Requirements)

* **RAM:** Tối thiểu **1 GB RAM** trở lên cho máy chủ/container chạy Backend.
  > [!WARNING]
  > **Không khuyến cáo** sử dụng cấu hình RAM 512 MB (như gói Free của Render hoặc máy ảo siêu nhỏ). Khi chạy các tác vụ nặng như trích xuất ảnh thumbnail bằng `ffmpeg` hoặc tải video dung lượng lớn từ URL, hệ thống có thể bị cạn kiệt bộ nhớ và tự động tắt (crash).

---

## 🚀 Hướng Dẫn Deploy Lên Render & Vercel

Hướng dẫn này giả định bạn sẽ triển khai **Backend trên Render** (sử dụng Docker) và **Frontend trên Vercel**.

### 📋 Chuẩn Bị Trước (Prerequisites)

1. **Cloudflare R2 Bucket:**
   * Tạo 1 Bucket trên Cloudflare R2 (ví dụ tên: `my-video-bucket`).
   * Vào mục **Manage R2 API Tokens** tạo một API Token mới có quyền `Read/Write` để lấy:
     * `R2_ACCOUNT_ID`
     * `R2_ACCESS_KEY_ID`
     * `R2_SECRET_ACCESS_KEY`
   * Thêm CORS Policy cho R2 Bucket để cấu hình CORS bảo mật (chỉ cho phép Frontend của bạn truy cập):
     ```json
     [
       {
         "AllowedHeaders": ["*"],
         "AllowedMethods": ["GET", "PUT", "HEAD"],
         "AllowedOrigins": [
           "https://your-app.vercel.app",
           "http://localhost:5173"
         ],
         "ExposeHeaders": []
       }
     ]
     ```

2. **Google OAuth Client ID:**
   * Truy cập [Google Cloud Console](https://console.cloud.google.com/).
   * Tạo một dự án mới và tạo thông tin xác thực **OAuth Client ID** loại **Web Application**.
   * Thêm địa chỉ tên miền Frontend (Vercel) và `http://localhost:5173` vào mục **Authorized JavaScript origins**.

---

### 📦 1. Triển Khai Backend Lên Render (Docker-based)

Dự án đã có sẵn `Dockerfile` đa tầng (multi-stage) tối ưu ở thư mục gốc. Render sẽ tự động phát hiện và build Docker image này.

1. Đăng nhập vào [Render](https://render.com/).
2. Nhấp vào **New +** và chọn **Web Service**.
3. Kết nối với repository GitHub này của bạn.
4. Cấu hình dịch vụ:
   * **Name:** `safe-stream-backend`
   * **Region:** Chọn vùng gần bạn nhất (ví dụ: `Singapore` hoặc `Oregon`).
   * **Language:** Chọn **Docker** (Render sẽ tự động dùng file `Dockerfile` ở thư mục gốc).
   * **Branch:** `main`
5. Thêm các biến môi trường (**Environment Variables**) trong tab **Env**:

| Tên Biến | Giá Trị Mẫu | Mô Tả |
| :--- | :--- | :--- |
| `PORT` | `8080` | Cổng dịch vụ chạy trên container Render (mặc định) |
| `R2_ACCOUNT_ID` | `abc123xyz...` | ID tài khoản Cloudflare R2 của bạn |
| `R2_ACCESS_KEY_ID` | `flashkey123...` | Access Key ID của R2 API Token |
| `R2_SECRET_ACCESS_KEY`| `secretkey456...` | Secret Access Key của R2 API Token |
| `R2_BUCKET_NAME` | `my-video-bucket` | Tên R2 bucket lưu trữ video |
| `GOOGLE_CLIENT_ID` | `xxxx.apps.googleusercontent.com`| Client ID tạo từ Google Console |
| `ALLOWED_EMAIL` | `your-email@gmail.com` | Email duy nhất được phép đăng nhập ứng dụng |
| `ALLOWED_ORIGIN` | `https://your-app.vercel.app` | URL Frontend trên Vercel (hoặc `*` nếu dev) |

6. Nhấn **Create Web Service**. Đợi Render build Docker image và deploy (khoảng 3-5 phút). Sau khi hoàn tất, bạn sẽ nhận được một URL backend dạng `https://safe-stream-backend.onrender.com`.

---

### 💻 2. Triển Khai Frontend Lên Vercel

1. Đăng nhập vào [Vercel](https://vercel.com/).
2. Nhấp vào **Add New...** -> **Project** và nhập repo này từ GitHub.
3. Cấu hình dự án:
   * **Framework Preset:** Chọn **Vite**.
   * **Root Directory:** Chọn thư mục `frontend`.
4. Cấu hình build commands (mặc định của Vite):
   * Build Command: `npm run build` hoặc `vite build`
   * Output Directory: `dist`
5. Thêm biến môi trường trong phần **Environment Variables**:
   * **Key:** `VITE_API_BASE_URL`
   * **Value:** Điền URL Backend Render bạn vừa deploy ở bước trên (ví dụ: `https://safe-stream-backend.onrender.com`).
6. Nhấp vào **Deploy**. Vercel sẽ build dự án chỉ trong vòng 1 phút và cung cấp cho bạn một domain public dạng `https://your-app.vercel.app`.
7. **Lưu ý:** Hãy quay lại Render phần cài đặt biến môi trường và cập nhật `ALLOWED_ORIGIN` thành tên miền Vercel này để tránh lỗi CORS khi truy cập.

---

### ☁️ Triển Khai Backend Lên Google Cloud Run (GCP)

Nếu bạn muốn deploy Backend lên môi trường Serverless tự động mở rộng và tối ưu chi phí của Google Cloud Platform (GCP) thay vì Render, dự án đã cấu hình sẵn quy trình CI/CD qua GitHub Actions.

#### 📋 Cần chuẩn bị trên Google Cloud:
1. **Google Cloud Project**: Tạo hoặc chọn một GCP Project và lấy ID (`GCP_PROJECT_ID`).
2. **Kích hoạt các API**: Bật dịch vụ **Cloud Run** và **Artifact Registry** trong GCP Console.
3. **Tạo Service Account**:
   * Tạo một Service Account (ví dụ: `github-actions-deployer`).
    * Cấp 4 quyền (roles) sau cho Service Account:
      * `Cloud Run Admin` (`roles/run.admin`) để triển khai và cấu hình dịch vụ Cloud Run.
      * `Storage Admin` (`roles/storage.admin`) để quản lý lưu trữ container images và các tài nguyên liên quan (cần thiết để lưu trữ và quản lý dữ liệu Docker).
      * `Artifact Registry Administrator` (`roles/artifactregistry.admin`) để kiểm tra, tự động tạo repository mới và push Docker image.
      * `Service Account User` (`roles/iam.serviceAccountUser`) để gán tài khoản chạy ứng dụng cho dịch vụ Cloud Run.
    * Tạo và tải về **Service Account Key** dưới định dạng JSON.

#### ⚙️ Cấu hình GitHub Secrets:
Truy cập vào **Settings** -> **Secrets and variables** -> **Actions** của Repository trên GitHub và thêm các biến mật khẩu sau:
* `GCP_PROJECT_ID`: ID dự án Google Cloud của bạn.
* `GCP_REGION`: Vùng đặt Cloud Run (ví dụ: `asia-southeast1` cho Singapore).
* `GCP_SA_KEY`: Toàn bộ nội dung file JSON Service Account Key đã tải xuống.

#### 🔄 Kích hoạt Tự Động Deploy:
1. Đổi tên file cấu hình workflow ở thư mục gốc từ `.github/workflows/deploy.yml.disabled` thành `.github/workflows/deploy.yml`.
2. Commit và push file này lên nhánh `main`. GitHub Actions sẽ tự động kích hoạt, build Docker image và triển khai ứng dụng lên Cloud Run.
3. **Cấu hình biến môi trường trên Cloud Run**:
   * Sau khi triển khai lần đầu, truy cập trang quản trị Cloud Run, chọn dịch vụ của bạn.
   * Nhấp vào **Edit & Deploy New Revision**.
   * Ở mục **Variables & Secrets**, thêm các biến môi trường cấu hình R2 và Google Auth (`R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`, `R2_SECRET_ACCESS_KEY`, `R2_BUCKET_NAME`, `GOOGLE_CLIENT_ID`, `ALLOWED_EMAIL`, `ALLOWED_ORIGIN`) tương tự như phần Render.

---

## 💻 Chạy Local (Local Development)

Nếu bạn muốn chạy thử nghiệm và phát triển ứng dụng ở máy cá nhân (Local):

### 1. Cấu hình Env
* Copy file `.env.example` thành `.env` ở cả 2 thư mục `backend/` và `frontend/`:
  * Ở thư mục `backend/`: cấu hình đầy đủ thông tin R2 và Google Client ID thực tế của bạn.
  * Ở thư mục `frontend/`: đặt `VITE_API_BASE_URL=http://localhost:8080`.

### 2. Khởi chạy Backend (Rust)
Đảm bảo máy bạn đã cài Rust và công cụ `ffmpeg`.
```bash
cd backend
cargo run
```
Dịch vụ sẽ khởi chạy tại cổng `http://localhost:8080`.

### 3. Khởi chạy Frontend (React Vite)
```bash
cd frontend
npm install
npm run dev
```
Trang web sẽ chạy tại địa chỉ `http://localhost:5173`.
