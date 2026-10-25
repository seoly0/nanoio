# ==========================================
# 1. Frontend Build Stage (React Web Console) - Currently Disabled
# ==========================================
# FROM node:20-alpine AS web-builder
# WORKDIR /app/web
# COPY nanoio-web/package*.json ./
# RUN npm ci
# COPY nanoio-web ./
# RUN npm run build

# ==========================================
# 2. Backend Build Stage (Rust S3 Server)
# ==========================================
FROM rust:slim-bookworm AS server-builder

WORKDIR /app

# Web UI 빌드 제외 시 rust-embed 참조를 위한 빈 디렉터리 생성
# COPY --from=web-builder /app/web/dist ./nanoio-web/dist
RUN mkdir -p ./nanoio-web/dist

# Server 소스 복사 및 바이너리 빌드
COPY nanoio-server ./nanoio-server
WORKDIR /app/nanoio-server
RUN cargo build --release

# ==========================================
# 3. Final Minimal Runtime Stage
# ==========================================
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=server-builder /app/nanoio-server/target/release/nanoio /usr/local/bin/nanoio
RUN ln -s /usr/local/bin/nanoio /usr/local/bin/open-object-storage

ENV NANOIO_HOST=0.0.0.0
ENV NANOIO_PORT=9000
ENV NANOIO_NO_CONSOLE=true
ENV NANOIO_DATA_DIR=/data
ENV RUST_LOG=info

VOLUME ["/data"]

# 9000: S3 REST API
# 9001: Web Management Console (Disabled)
EXPOSE 9000

ENTRYPOINT ["nanoio"]

