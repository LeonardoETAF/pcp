//! Parâmetros estatísticos de estoque por produto (doc 02 §3): janela de 12 meses só com
//! dias de venda, remoção de outliers por IQR, média/desvio e o estoque recomendado
//! UNIFICADO na fórmula meta-ABC (§3.6 + segurança z e teto de 60 dias da §3.5).

mod calculo;
pub(crate) mod estatistica;
mod formula;

pub use calculo::{
    calcular_parametros, DefaultsSemHistorico, ParametrosEstoque, ParametrosEstoqueConfig,
    StatusParametros,
};
pub use estatistica::VendaDiaria;
