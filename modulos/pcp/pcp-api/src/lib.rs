//! Servidor HTTP do **módulo PCP** (Axum): endpoints sob `/pcp/...` (CLAUDE.md §2/§7).
//!
//! Autenticação (argon2 + JWT + refresh revogável), autorização por papel e o erro HTTP
//! **não vivem aqui** — são do núcleo (`sf-auth`/`sf-http`), compartilhados por todos os
//! módulos do `SuperFlow` (§0). Este crate só os consome e liga ao seu [`AppState`].
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

pub mod bootstrap;
mod ciclo_vida;
mod config;
mod estado;
mod filtros_salvos;
mod handlers_auth;
mod handlers_pcp;
mod leitura;
mod preferencias;
mod recomendacao;
mod rotas;
mod sazonalidade;
mod solicitacoes;
pub mod tempo_real;
mod usuarios;

pub use estado::{AppState, ConfigApi, ErroBootstrap};
pub use rotas::rotas;
pub use sf_auth::Papel;
pub use sf_http::ApiError;
pub use tempo_real::escutar_pipeline;
