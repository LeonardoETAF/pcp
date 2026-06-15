//! ETL do PCP: ingestão dos dados de entrada atrás do trait `FonteDados` (CLAUDE.md §1/§8).
//! Hoje: importação de arquivo (CSV/dump); amanhã: conector ao ERP "One", sem mudar o motor.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

mod arquivo;
mod erro;
mod fonte;
mod importacao;

pub use arquivo::ImportadorArquivo;
pub use erro::ErroEtl;
pub use fonte::FonteDados;
pub use importacao::{importar, ResumoImportacao};
