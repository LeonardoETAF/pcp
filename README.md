# PCP — Planejamento e Controle de Produção (SuperCopo)

Reconstrução do PCP em **Rust 100%** (backend e frontend), pensado como um **módulo do
futuro ERP**. O contrato de desenvolvimento é [CLAUDE.md](CLAUDE.md); as regras de negócio
canônicas estão em [docs/prd/02-regras-de-negocio.md](docs/prd/02-regras-de-negocio.md).

## Stack (fixa — CLAUDE.md §1)

Rust · Leptos (SSR + WASM) · Axum · SQLx · PostgreSQL (Docker) · Anthropic Claude ·
ETL nativo · Auth própria (JWT + papéis) · pt-BR.

## Workspace (CLAUDE.md §2)

Dependência one-way, núcleo no centro: `pcp-core` não depende de nada do projeto.

| Crate | Responsabilidade | Depende de |
|---|---|---|
| `pcp-core` | Domínio puro: regras do doc 02. Sem I/O. | — |
| `pcp-config` | Carrega/valida `pcp.config.yaml` + auditoria. | — |
| `pcp-db` | Repositórios SQLx, modelos de persistência. | core |
| `pcp-etl` | Ingestão (arquivo/CSV; ERP depois). | core, db |
| `pcp-engine` | Motor diário: orquestra os 4 módulos. | core, config, db |
| `pcp-ai` | Chat IA, análise por produto, insights. | core, config, db |
| `pcp-api` | Servidor Axum: auth, autorização, endpoints. | core, config, db, engine, ai |
| `pcp-web` | Frontend Leptos. Nunca importa regra. | — (consome a API) |

Testes de paridade/invariantes de regra: crate `tests/` (CLAUDE.md §2/§11).

## Desenvolvimento

```bash
cargo build                      # compila o workspace
cargo fmt --check                # formatação
cargo clippy -- -D warnings      # lint (sem warnings)
cargo test                       # testes
```

Toolchain estável fixada em [rust-toolchain.toml](rust-toolchain.toml). **Sem `unsafe`**
em todo o projeto (`#![forbid(unsafe_code)]`).

## Banco de dados (local)

Postgres **dedicado** via Docker, na porta do host **5433** por padrão (evita colidir com
um Postgres local em 5432). Definição em [docker-compose.yml](docker-compose.yml).

```bash
cp .env.example .env          # ajuste POSTGRES_PASSWORD / DATABASE_URL
docker compose up -d --wait   # sobe o Postgres dedicado e espera ficar saudável
sqlx migrate run              # aplica as migrations de migrations/ (requer sqlx-cli)
```

- **Schema:** todas as tabelas vivem em `pcp.*` (CLAUDE.md §0); migrations versionadas em
  [migrations/](migrations/). Política de retenção semeada em `pcp.retencao_politica` (§9).
- **SQLx em compile-time:** as queries são verificadas contra o banco. Para builds/CI **sem
  banco**, há um cache offline em `.sqlx/` (versionado; regere com
  `cargo sqlx prepare --workspace` após mudar qualquer query).
- **Testes de banco:** os testes de integração de `pcp-db` precisam do Postgres e estão
  marcados `#[ignore]`. Rode com `cargo test -p pcp-db -- --ignored`.

> Secrets só em `.env` (fora do git) — nunca versione credenciais (CLAUDE.md §7.4).
