//! Estrutura tipada da configuração de negócio (doc 02 §11 / CLAUDE.md §12).
//! Espelha exatamente `config/pcp.config.yaml`. Apenas dados — sem lógica.

use serde::{Deserialize, Serialize};

/// Configuração completa do PCP, carregada de `config/pcp.config.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub classificacao: Classificacao,
    pub metas_cobertura_dias: MetasCobertura,
    pub limiar_critico_dias: LimiarCritico,
    pub parametros_estoque: ParametrosEstoque,
    pub sazonalidade: Sazonalidade,
    pub alertas: Alertas,
    pub reposicao: Reposicao,
    pub fora_de_linha: ForaDeLinha,
    pub metas_estoque_fisico_pct: MetasEstoqueFisico,
    /// Operação do pipeline (doc 05 §3). `default` para não quebrar config antiga.
    #[serde(default)]
    pub pipeline: Pipeline,
    /// Estado de produção exibido na UI (decisão do dono, 2026-07-13).
    #[serde(default)]
    pub producao: Producao,
}

/// Estado de produção exibido na lista e no detalhe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Producao {
    /// Dias em que uma ordem FINALIZADA ainda conta como "recém produzido".
    pub recem_produzido_dias: u32,
}

impl Default for Producao {
    fn default() -> Self {
        Self {
            recem_produzido_dias: 2,
        }
    }
}

/// Parâmetros de operação do pipeline diário (doc 05 §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pipeline {
    /// Quantos dias antes de `data_ref` a pré-validação aceita procurar vendas. Existe porque
    /// "vendas de ontem" é falso em segunda-feira e depois de feriado: sem tolerância, o pipeline
    /// bloquearia toda semana. 1 = exige literalmente o dia anterior.
    pub tolerancia_vendas_dias: u32,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            tolerancia_vendas_dias: 3,
        }
    }
}

/// Parâmetros da classificação ABC+F+D+N (doc 02 §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Classificacao {
    pub janela_abc_dias: u32,
    pub janela_classe_d_dias: u32,
    pub janela_produto_novo_dias: u32,
    pub pareto_a: u8,
    pub pareto_b: u8,
    pub fator_estoque: FatorEstoque,
}

/// Fator multiplicador de estoque por classe (doc 02 §2.5).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "UPPERCASE")]
pub struct FatorEstoque {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub f: f64,
    pub n: f64,
}

/// Meta de cobertura em dias por classe (doc 02 §3.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "UPPERCASE")]
pub struct MetasCobertura {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
    pub f: u32,
    pub n: u32,
    #[serde(rename = "default")]
    pub default: u32,
}

/// Limiar de criticidade (cobertura em dias) por classe (doc 02 §5.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "UPPERCASE")]
pub struct LimiarCritico {
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

/// Parâmetros estatísticos de estoque (doc 02 §3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParametrosEstoque {
    pub janela_vendas_meses: u32,
    pub min_dias_com_vendas: u32,
    pub outlier_iqr_mult: f64,
    pub z_score_seguranca: f64,
    pub dias_base_minimo: u32,
    pub teto_cobertura_dias: u32,
    /// Estoque mínimo = `fracao_minimo` × alvo-meta (doc 02 §3.6: 0.70).
    pub fracao_minimo: f64,
    /// Meia-vida do decaimento por recência, em dias. 0 desliga (decisão do dono, 2026-07-13).
    #[serde(default = "meia_vida_padrao")]
    pub meia_vida_dias: f64,
    pub defaults_sem_historico: DefaultsSemHistorico,
}

/// Default da meia-vida do decaimento por recência (dias).
fn meia_vida_padrao() -> f64 {
    90.0
}

/// Valores default para produtos sem histórico confiável (doc 02 §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DefaultsSemHistorico {
    pub media: f64,
    pub min: u32,
    pub seguranca: u32,
    pub recomendado_max: u32,
}

/// Sazonalidade dinâmica (doc 02 §4).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sazonalidade {
    pub clamp_min: f64,
    pub clamp_max: f64,
    pub atualizar_apos_dias: u32,
    /// Mínimo de meses com venda para o produto ter curva sazonal própria (default 6).
    #[serde(default = "min_meses_padrao")]
    pub min_meses_com_venda_produto: u32,
}

/// Default do mínimo de meses com venda para curva sazonal própria (doc 02 §4, por produto).
fn min_meses_padrao() -> u32 {
    6
}

/// Limiares dos alertas de produção (doc 02 §6).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Alertas {
    pub critico_pct: f64,
    pub alto_pct: f64,
    pub medio_pct: f64,
    pub elevar_classe_a: bool,
}

/// Recomendação de produção/reposição (doc 02 §7).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reposicao {
    pub fator_urgencia: FatorUrgencia,
    pub protecao_ruptura_dias: u32,
    pub aprovacao_automatica: AprovacaoAutomatica,
    pub lead_time_dias: LeadTime,
}

/// Fator de urgência por faixa de cobertura (doc 02 §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FatorUrgencia {
    pub cobertura_lt_7: f64,
    pub cobertura_lt_15: f64,
    pub default: f64,
}

/// Política de aprovação automática de produção (doc 02 §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AprovacaoAutomatica {
    pub qtd_max: u32,
    pub exceto_prioridade: Prioridade,
}

/// Lead time de produção em dias por prioridade (doc 02 §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LeadTime {
    pub alta: u32,
    pub media: u32,
    pub baixa: u32,
}

/// Prioridade de uma solicitação de produção (doc 02 §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Prioridade {
    Alta,
    Media,
    Baixa,
}

/// Análise de fora de linha / ciclo de vida (doc 02 §8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForaDeLinha {
    pub limiar_sugerir_saida: u32,
    pub limiar_sugerir_volta: u32,
    pub alta_certeza: u32,
    pub media_certeza: u32,
}

/// Meta de participação no estoque físico por classe, em % (doc 02 §9.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "UPPERCASE")]
pub struct MetasEstoqueFisico {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}
