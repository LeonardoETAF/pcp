//! **Núcleo do `SuperFlow` — infra de dados.** Compartilhado por TODOS os módulos
//! (PCP, Catálogo, ...). Contém o que não pertence a nenhum módulo em particular:
//! pool de conexões, erro tipado, migrations do núcleo e a **identidade** (usuário e
//! refresh token, no schema `nucleo`) — CLAUDE.md §0/§15.
//!
//! Nenhuma regra de negócio de módulo vive aqui.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

mod erro;
mod migracoes;
mod pool;

pub mod identidade;

pub use erro::ErroDb;
pub use identidade::{refresh_tokens, usuarios, RefreshToken, Usuario};
pub use migracoes::aplicar_migrations_nucleo;
pub use pool::criar_pool;
pub use sqlx::PgPool;
