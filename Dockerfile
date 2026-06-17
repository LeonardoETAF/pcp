# syntax=docker/dockerfile:1
# Build multi-stage do PCP (100% Rust — CLAUDE.md §1). Um builder compartilhado compila o backend
# Axum (pcp-api) e o frontend SSR Leptos (pcp-web + assets WASM/CSS); dois alvos de runtime enxutos.
# SQLx em modo offline usa o cache `.sqlx` versionado (sem precisar de banco no build).
#
# Uso (compose escolhe o alvo):  docker build --target pcp-api .   |   --target pcp-web .

# ---------- builder ----------
FROM rust:1-bookworm AS builder
WORKDIR /app
ENV SQLX_OFFLINE=true
# Alvo WASM + cargo-leptos (baixa wasm-bindgen/wasm-opt/sass na medida do necessário).
RUN rustup target add wasm32-unknown-unknown \
 && cargo install cargo-leptos --locked
COPY . .
# Backend.
RUN cargo build --release -p pcp-api
# Frontend SSR + WASM (perfis wasm-release/server-release definidos no Cargo.toml do workspace).
RUN cargo leptos build --release

# ---------- runtime: API (backend Axum) ----------
FROM debian:bookworm-slim AS pcp-api
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/pcp-api /usr/local/bin/pcp-api
ENV PCP_API_BIND=0.0.0.0:8080
EXPOSE 8080
# Aplica as migrations embutidas no start (pcp-db) e sobe o servidor.
ENTRYPOINT ["pcp-api"]

# ---------- runtime: WEB (SSR Leptos + assets) ----------
FROM debian:bookworm-slim AS pcp-web
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/server-release/pcp-web /usr/local/bin/pcp-web
COPY --from=builder /app/target/site /app/site
# O binário SSR lê estas variáveis (get_configuration por ambiente).
ENV LEPTOS_OUTPUT_NAME=pcp-web \
    LEPTOS_SITE_ROOT=/app/site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_SITE_ADDR=0.0.0.0:3000
EXPOSE 3000
ENTRYPOINT ["pcp-web"]
