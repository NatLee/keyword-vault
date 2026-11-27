# Build targets: runtime (internal API), frontend (nginx), standalone (default).

# 1. 前端
FROM node:24-slim AS web
ENV COREPACK_ENABLE_DOWNLOAD_PROMPT=0
WORKDIR /web
RUN corepack enable
COPY src/web/package.json src/web/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY src/web/ ./
RUN pnpm build

# 2. 後端
FROM rust:1-trixie AS api
WORKDIR /workspace
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY src/api ./src/api
RUN cargo build -p keyword-vault --release --locked --jobs 2

# 3. Internal, nonroot API runtime
FROM debian:trixie-slim AS runtime
RUN useradd --uid 10001 --create-home --home-dir /home/app --shell /usr/sbin/nologin app \
    && mkdir -p /app/data \
    && chown app /app/data
WORKDIR /app
COPY --from=api /workspace/target/release/keyword-vault /usr/local/bin/keyword-vault
ENV KV_BIND=0.0.0.0:8686 \
    KV_DATA_DIR=/app/data \
    KV_STATIC_DIR=/app/web
VOLUME /app/data
EXPOSE 8686
USER app
CMD ["keyword-vault"]

# 4. Compose public entry point: nginx serves the built SPA and proxies the API.
FROM nginx:stable-alpine AS frontend
COPY --from=web /web/dist /usr/share/nginx/html
COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf
COPY deploy/nginx-proxy.conf /etc/nginx/snippets/proxy.conf
COPY deploy/nginx-static-headers.conf /etc/nginx/snippets/static-headers.conf

# 5. Preserve the standalone backend + static image for existing CI/direct runs.
FROM runtime AS standalone
COPY --from=web /web/dist /app/web
