//! Motor diário do PCP: orquestra os módulos (classificação → parâmetros → alertas → fora
//! de linha) sobre `pcp-core` e `pcp-db`, de forma idempotente por `data_ref` (doc 05 §1.2).
//! Aqui moram I/O e orquestração; a regra pura vive no `pcp-core` (CLAUDE.md §2/§3).
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

mod erro;
mod mapeamento;
mod pipeline;
pub mod sazonalidade;
mod webhook;

pub use erro::ErroEngine;
pub use pipeline::{processar_dia, reprocessar_intervalo, ResultadoPipeline, StatusPipeline};
