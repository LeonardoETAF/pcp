# syntax=docker/dockerfile:1
# Build multi-stage do PCP (100% Rust — CLAUDE.md §1). Um builder compartilhado compila o backend
# Axum (pcp-api) e o frontend SSR Leptos (pcp-web + assets WASM/CSS); dois alvos de runtime enxutos.
# SQLx em modo offline usa o cache `.sqlx` versionado (sem precisar de banco no build).
#
# Uso (compose escolhe o alvo):  --target pcp-api | --target pcp-web | --target pcp-sync

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
# ETL: sincronização contínua com o One (sync_one) e backfill histórico (backfill_one).
RUN cargo build --release -p pcp-etl --bins
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

# ---------- runtime: SYNC (ingestão contínua do ERP One) ----------
# Loop de polling: One (read-only) → bronze → ACL → pipeline do dia → NOTIFY → SSE (doc 05, §16).
# Traz também o `backfill_one` para a carga histórica inicial:
#   docker compose -f docker-compose.prod.yml run --rm --entrypoint backfill_one sync-one
FROM debian:bookworm-slim AS pcp-sync
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/target/release/sync_one /usr/local/bin/sync_one
COPY --from=builder /app/target/release/backfill_one /usr/local/bin/backfill_one
# Constantes de negócio (doc 02 §11) lidas pelo motor — nunca hardcoded (§3.7).
COPY config /app/config
ENV PCP_CONFIG_PATH=/app/config/pcp.config.yaml
ENTRYPOINT ["sync_one"]

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
