//! Domínio puro do PCP: todas as regras do doc 02 (classificação, parâmetros, alertas,
//! ciclo de vida, sazonalidade). Funções puras, sem relógio/banco/rede — o tempo entra
//! como parâmetro (`data_ref`). Os limiares vêm de pcp-config via o chamador, nunca
//! importados aqui (CLAUDE.md §2/§3/§5).
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

pub mod classificacao;
mod consolidacao;
mod tipos;

pub use classificacao::{
    classificar, FatoresAbc, ParametrosClassificacao, ProdutoParaClassificar,
    ResultadoClassificacao,
};
pub use consolidacao::{consolidar, VendaBruta, VendaConsolidada};
pub use tipos::{ClasseAbc, CodigoEstoque};
