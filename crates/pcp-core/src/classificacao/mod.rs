//! Classificação ABC+F+D+N (doc 02 §2): precedência F → D → N → Pareto, executada para uma
//! `data_ref`. Funções puras; limiares recebidos via [`ParametrosClassificacao`].

mod classificador;
mod parametros;
mod pareto;
mod precedencia;

pub use classificador::{classificar, ProdutoParaClassificar, ResultadoClassificacao};
pub use parametros::{FatoresAbc, ParametrosClassificacao};
