//! ETL do PCP: ingestão dos dados de entrada atrás do trait `FonteDados` (CLAUDE.md §1/§8).
//! Importação de arquivo (CSV/dump) para backfill e conector somente-leitura ao ERP One
//! (`FonteConsultaOne`), sem mudar o motor: a persistência (`gravar`) é única.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

mod arquivo;
mod bronze;
mod complementar;
mod erro;
mod fonte;
mod importacao;
mod one;

pub use arquivo::ImportadorArquivo;
pub use bronze::{acl_estoque, acl_venda, BronzeEstoque, BronzeVenda};
pub use erro::ErroEtl;
pub use fonte::FonteDados;
pub use importacao::{gravar, importar, ResumoImportacao};
pub use one::{FonteConsultaOne, OpcoesOne};
