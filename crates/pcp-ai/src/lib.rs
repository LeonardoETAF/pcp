//! IA: chat com tool use (somente leitura), análise por produto com fallback local e
//! insights estatísticos no backend, via Anthropic Claude. (CLAUDE.md §10)
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

pub mod estatistica;
pub mod insights;

pub use insights::{analisar, AlertaInteligente, ContextoProduto, Insights, PontoVenda};
