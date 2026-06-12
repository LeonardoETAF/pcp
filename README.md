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
