# ==========================================
# Stage 1: Build Frontend (React + TypeScript)
# ==========================================
FROM node:20-alpine AS frontend-builder
WORKDIR /app/frontend

# Sao chép package.json và cài đặt dependencies
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm install

# Sao chép toàn bộ mã nguồn frontend và build
COPY frontend/ ./
RUN npm run build

# ==========================================
# Stage 2: Build Backend (Rust Axum)
# ==========================================
FROM rust:slim-bookworm AS backend-builder
WORKDIR /app/backend

# Cài đặt pkg-config và openssl để biên dịch AWS SDK và các thư viện cần thiết
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Sao chép Cargo.toml và tải xuống/biên dịch dependencies trước để cache (tối ưu tốc độ build)
COPY backend/Cargo.toml backend/Cargo.lock* ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# Sao chép toàn bộ mã nguồn thực tế và biên dịch lại bản release
COPY backend/ ./
RUN touch src/main.rs && cargo build --release

# ==========================================
# Stage 3: Runtime Environment (Tối giản)
# ==========================================
FROM debian:bookworm-slim AS runner
WORKDIR /app

# Cài đặt chứng chỉ SSL và ffmpeg để trích xuất ảnh thumbnail
RUN apt-get update && apt-get install -y \
    ca-certificates \
    openssl \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Sao chép file chạy từ backend-builder
COPY --from=backend-builder /app/backend/target/release/video-streaming-backend /app/video-server

# Sao chép thư mục frontend tĩnh được build từ frontend-builder
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist

# Expose cổng mặc định
EXPOSE 8080

# Chạy server
CMD ["/app/video-server"]
