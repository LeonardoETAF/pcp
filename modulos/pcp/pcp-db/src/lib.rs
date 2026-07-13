//! Acesso a dados do **módulo PCP** (schema `pcp`): pool, migrations do módulo, modelos de
//! persistência e repositórios. Escrita só pelo pipeline; sem regra de negócio aqui
//! (CLAUDE.md §6/§7).
//!
//! Identidade (usuário/refresh token) **não vive aqui** — é do núcleo (`sf-db`), porque é
//! compartilhada por todos os módulos do `SuperFlow` (§0).
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

mod erro;
mod migracoes;
mod modelos;
mod pool;

pub mod agregacoes;
pub mod atividade_produto;
pub mod ciclo_vida;
pub mod config_persist;
pub mod derivadas;
pub mod detalhe;
pub mod eventos;
pub mod expurgo;
pub mod filtros_salvos;
pub mod leituras;
pub mod operacao;
pub mod preferencias;
pub mod sazonalidade;
pub mod snapshot;
pub mod solicitacoes;
pub mod vendas;

pub use erro::ErroDb;
pub use expurgo::expurgar;
pub use migracoes::aplicar_migrations;
pub use modelos::{EstoqueSnapshot, NovaVendaDia, NovoEstoqueSnapshot, VendaDia};
pub use pool::criar_pool;
pub use sqlx::PgPool;
