//! Cola entre a config de negócio e o serviço único de recomendação do `pcp-core` (doc 02 §7).
//! Só traduz config↔tipos do core — a regra vive no `pcp-core` (CLAUDE.md §3.1).

use pcp_config::Config;
use pcp_core::{ClasseAbc, ParametrosRecomendacao, PrioridadeProducao};

/// Meta de cobertura (dias) da classe, vinda da config (doc 02 §3.6 / §11).
#[must_use]
pub fn meta_cobertura(c: &Config, classe: &str) -> u32 {
    let m = &c.metas_cobertura_dias;
    match classe {
        "A" => m.a,
        "B" => m.b,
        "C" => m.c,
        "D" => m.d,
        "F" => m.f,
        "N" => m.n,
        _ => m.default,
    }
}

/// Monta os [`ParametrosRecomendacao`] do `pcp-core` a partir da config e da classe (doc 02 §7).
#[must_use]
pub fn parametros(c: &Config, classe: &str) -> ParametrosRecomendacao {
    ParametrosRecomendacao {
        meta_dias_classe: i64::from(meta_cobertura(c, classe)),
        fator_urgencia_lt7: c.reposicao.fator_urgencia.cobertura_lt_7,
        fator_urgencia_lt15: c.reposicao.fator_urgencia.cobertura_lt_15,
        fator_urgencia_default: c.reposicao.fator_urgencia.default,
        protecao_ruptura_dias: i64::from(c.reposicao.protecao_ruptura_dias),
        dias_base_minimo: i64::from(c.parametros_estoque.dias_base_minimo),
        lead_time_alta: i64::from(c.reposicao.lead_time_dias.alta),
        lead_time_media: i64::from(c.reposicao.lead_time_dias.media),
        lead_time_baixa: i64::from(c.reposicao.lead_time_dias.baixa),
    }
}

/// Classe a partir do texto persistido (default `N` se desconhecido — não deveria ocorrer).
#[must_use]
pub fn classe(texto: &str) -> ClasseAbc {
    ClasseAbc::tentar_de_str(texto).unwrap_or(ClasseAbc::N)
}

/// Código textual da prioridade (persistência/JSON).
#[must_use]
pub fn prioridade_str(p: PrioridadeProducao) -> &'static str {
    match p {
        PrioridadeProducao::Alta => "alta",
        PrioridadeProducao::Media => "media",
        PrioridadeProducao::Baixa => "baixa",
    }
}

/// Prioridade a partir do texto; `None` se desconhecido.
#[must_use]
pub fn prioridade_de(texto: &str) -> Option<PrioridadeProducao> {
    match texto {
        "alta" => Some(PrioridadeProducao::Alta),
        "media" => Some(PrioridadeProducao::Media),
        "baixa" => Some(PrioridadeProducao::Baixa),
        _ => None,
    }
}

/// Prioridade-exceção da aprovação automática, vinda da config (doc 02 §7.2).
#[must_use]
pub fn excecao_aprovacao(c: &Config) -> PrioridadeProducao {
    match c.reposicao.aprovacao_automatica.exceto_prioridade {
        pcp_config::Prioridade::Alta => PrioridadeProducao::Alta,
        pcp_config::Prioridade::Media => PrioridadeProducao::Media,
        pcp_config::Prioridade::Baixa => PrioridadeProducao::Baixa,
    }
}
