//! Servidor HTTP do PCP (Axum): autenticação própria (argon2 + JWT + refresh revogável),
//! autorização por papel deny-by-default e endpoints sob `/pcp/...` (CLAUDE.md §2/§7).
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

mod autenticacao;
pub mod bootstrap;
mod erro;
mod estado;
mod filtros_salvos;
mod handlers_auth;
mod handlers_pcp;
mod jwt;
mod leitura;
mod papel;
mod recomendacao;
mod rotas;
pub mod senha;
mod solicitacoes;
pub mod tempo_real;

pub use erro::ApiError;
pub use estado::{AppState, ConfigApi, ErroBootstrap};
pub use papel::Papel;
pub use rotas::rotas;
pub use tempo_real::escutar_pipeline;
