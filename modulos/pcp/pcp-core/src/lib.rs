//! Domínio puro do PCP: todas as regras do doc 02 (classificação, parâmetros, alertas,
//! ciclo de vida, sazonalidade). Funções puras, sem relógio/banco/rede — o tempo entra
//! como parâmetro (`data_ref`). Os limiares vêm de pcp-config via o chamador, nunca
//! importados aqui (CLAUDE.md §2/§3/§5).
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

mod alertas;
pub mod ciclo_vida;
pub mod classificacao;
mod consolidacao;
pub mod parametros;
mod recomendacao;
pub mod sazonalidade;
pub mod solicitacao;
mod status;
mod tipos;

pub use alertas::{gerar_alertas, Alerta, EntradaAlerta, ParametrosAlerta, Prioridade};
pub use ciclo_vida::{
    analisar, transicionar, AcaoSugerida, CriterioCicloVida, EntradaCicloVida, ErroTransicao,
    EstadoCicloVida, NivelCerteza, ParametrosCicloVida, SugestaoCicloVida,
};
pub use classificacao::{
    classificar, FatoresAbc, ParametrosClassificacao, ProdutoParaClassificar,
    ResultadoClassificacao,
};
pub use consolidacao::{consolidar, VendaBruta, VendaConsolidada};
pub use parametros::{
    calcular_parametros, DefaultsSemHistorico, ParametrosEstoque, ParametrosEstoqueConfig,
    StatusParametros, VendaDiaria,
};
pub use recomendacao::{
    aprovacao_automatica, qtd_sugerida, recomendar_producao, EntradaRecomendacao,
    ParametrosRecomendacao, PrioridadeProducao, RecomendacaoProducao, Timing,
};
pub use sazonalidade::{calcular_fator, deve_recalcular, FatoresSazonais, ParametrosSazonalidade};
pub use solicitacao::estado::EstadoSolicitacao;
pub use status::{
    cobertura_dias, status_estoque, EntradaStatus, LimiarCriticoDias, StatusEstoque,
    COBERTURA_SEM_HISTORICO,
};
pub use tipos::{ClasseAbc, CodigoEstoque};
