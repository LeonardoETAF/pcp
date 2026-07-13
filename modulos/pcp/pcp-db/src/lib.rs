//! Acesso a dados do PCP: pool de conexões, helper de migrations, modelos de persistência
//! e repositórios das tabelas de entrada (`pcp.vendas_dia`, `pcp.estoque_snapshot`).
//! Escrita só pelo pipeline; sem regra de negócio aqui (CLAUDE.md §6/§7).
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
pub mod refresh_tokens;
pub mod sazonalidade;
pub mod snapshot;
pub mod solicitacoes;
pub mod usuarios;
pub mod vendas;

pub use erro::ErroDb;
pub use expurgo::expurgar;
pub use migracoes::aplicar_migrations;
pub use modelos::{EstoqueSnapshot, NovaVendaDia, NovoEstoqueSnapshot, VendaDia};
pub use pool::criar_pool;
pub use refresh_tokens::RefreshToken;
pub use sqlx::PgPool;
pub use usuarios::Usuario;
