FROM node:22-alpine AS frontend
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY tsconfig.json vite.config.ts ./
COPY web/ ./web/
RUN npm run build

FROM rust:1-slim-bookworm AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src/ ./src/
COPY migrations/ ./migrations/
COPY build.rs ./
RUN cargo build --locked --release

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
RUN groupadd -r gxe && useradd -r -g gxe gxe && mkdir -p /app/data && chown -R gxe:gxe /app/data
COPY --from=builder /app/target/release/gxe-search-queue /usr/local/bin/gxe-search-queue
COPY --from=frontend /app/web-dist /app/web
USER gxe
ENV DATABASE_PATH=/app/data/gxe-search-queue.sqlite3
ENV RUST_LOG=info
ENV API_BIND_ADDR=0.0.0.0
ENV API_PORT=8080
HEALTHCHECK --interval=30s --timeout=5s --retries=3 CMD curl --fail http://127.0.0.1:8080/api/health || exit 1
ENTRYPOINT ["gxe-search-queue"]
