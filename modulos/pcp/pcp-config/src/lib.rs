//! Configuração de negócio editável do PCP: carrega/valida `config/pcp.config.yaml` com
//! todas as constantes do doc 02 §11, e define a porta de auditoria de mudanças.
//! Fonte ÚNICA das constantes — nada hardcoded em outros crates (CLAUDE.md §3.7/§13).
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

mod auditoria;
mod carregar;
mod erro;
mod modelo;
mod validacao;

pub use auditoria::{AuditoriaConfig, MudancaConfig};
pub use carregar::{carregar_de_arquivo, carregar_de_str};
pub use erro::ErroConfig;
pub use modelo::{
    Alertas, AprovacaoAutomatica, Classificacao, Config, DefaultsSemHistorico, FatorEstoque,
    FatorUrgencia, ForaDeLinha, LeadTime, LimiarCritico, MetasCobertura, MetasEstoqueFisico,
    ParametrosEstoque, Prioridade, Reposicao, Sazonalidade,
};
pub use validacao::validar;
