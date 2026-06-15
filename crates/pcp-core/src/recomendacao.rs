//! Serviço ÚNICO de recomendação de produção (doc 02 §7), unificando §7.1/§7.2/§7.3:
//! base meta-ABC sobre `qtd_disponivel` (doc 08 §1.8), × fator de urgência × fator sazonal,
//! com proteção de ruptura; o escalonamento por criticidade (§7.3) vira política de timing.
// Quantidades pequenas e não-negativas; os casts f64<->i64 são seguros.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use crate::tipos::ClasseAbc;

// Limiares estruturais do §7.2/§7.3 (não são tunables do §11; fazem parte da fórmula).
const COBERTURA_URGENCIA_ALTA_DIAS: f64 = 7.0; // doc 02 §7.2
const COBERTURA_URGENCIA_MEDIA_DIAS: f64 = 15.0; // doc 02 §7.2
const TIMING_IMEDIATO_FRACAO: f64 = 0.3; // doc 02 §7.3
const TIMING_UMA_SEMANA_FRACAO: f64 = 0.6; // doc 02 §7.3
const TIMING_AGUARDAR_FRACAO: f64 = 1.5; // doc 02 §7.3

/// Quantidade sugerida simples (doc 02 §7.1) — usada nos alertas e na tabela de estoque.
/// `MAX(0, recomendado − disponivel)`, ou 0 se fora de linha / sem histórico.
#[must_use]
pub fn qtd_sugerida(
    recomendado: i64,
    disponivel: i64,
    fora_de_linha: bool,
    media_diaria: f64,
) -> i64 {
    if fora_de_linha || media_diaria <= 0.0 {
        return 0;
    }
    (recomendado - disponivel).max(0)
}

/// Prioridade da solicitação de produção (doc 02 §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrioridadeProducao {
    Alta,
    Media,
    Baixa,
}

/// Política de timing (doc 02 §7.3 — escalonamento por criticidade).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Timing {
    Imediato,
    UmaSemana,
    DuasSemanas,
    Aguardar,
    Monitorar,
}

/// Dados de um produto para a recomendação de produção (doc 02 §7).
#[derive(Debug, Clone)]
pub struct EntradaRecomendacao {
    pub classe: ClasseAbc,
    pub media_diaria: f64,
    pub qtd_disponivel: i64,
    pub estoque_seguranca: i64,
    pub cobertura_dias: f64,
    pub fora_de_linha: bool,
    pub alerta_critico: bool,
}

/// Limiares configuráveis da recomendação (doc 02 §7/§11). Originados de pcp-config (§2/§13).
#[derive(Debug, Clone, Copy)]
pub struct ParametrosRecomendacao {
    pub meta_dias_classe: i64,
    pub fator_urgencia_lt7: f64,
    pub fator_urgencia_lt15: f64,
    pub fator_urgencia_default: f64,
    pub protecao_ruptura_dias: i64,
    pub dias_base_minimo: i64,
    pub lead_time_alta: i64,
    pub lead_time_media: i64,
    pub lead_time_baixa: i64,
}

/// Recomendação de produção (doc 02 §7).
#[derive(Debug, Clone, PartialEq)]
pub struct RecomendacaoProducao {
    pub qtd_final: i64,
    pub prioridade: PrioridadeProducao,
    pub timing: Timing,
    pub lead_time_dias: i64,
}

/// Recomendação de produção unificada (doc 02 §7). A quantidade vem da §7.2 (meta-ABC base ×
/// urgência × sazonal, com proteção de ruptura); o timing, da §7.3.
#[must_use]
pub fn recomendar_producao(
    entrada: &EntradaRecomendacao,
    fator_sazonal: f64,
    params: &ParametrosRecomendacao,
) -> RecomendacaoProducao {
    let prioridade = prioridade(entrada);
    let lead_time_dias = match prioridade {
        PrioridadeProducao::Alta => params.lead_time_alta,
        PrioridadeProducao::Media => params.lead_time_media,
        PrioridadeProducao::Baixa => params.lead_time_baixa,
    };
    let qtd_final = if entrada.fora_de_linha || entrada.media_diaria <= 0.0 {
        0
    } else {
        quantidade(entrada, fator_sazonal, params)
    };
    RecomendacaoProducao {
        qtd_final,
        prioridade,
        timing: timing(entrada.cobertura_dias, params.meta_dias_classe),
        lead_time_dias,
    }
}

fn quantidade(
    entrada: &EntradaRecomendacao,
    fator_sazonal: f64,
    params: &ParametrosRecomendacao,
) -> i64 {
    let estoque_ideal = (entrada.media_diaria * params.meta_dias_classe as f64).round();
    let qtd_necessaria =
        (estoque_ideal - entrada.qtd_disponivel as f64 + entrada.estoque_seguranca as f64).max(0.0);

    let fator_urgencia = if entrada.cobertura_dias < COBERTURA_URGENCIA_ALTA_DIAS {
        params.fator_urgencia_lt7
    } else if entrada.cobertura_dias < COBERTURA_URGENCIA_MEDIA_DIAS {
        params.fator_urgencia_lt15
    } else {
        params.fator_urgencia_default
    };
    let mut qtd_final = (qtd_necessaria * fator_urgencia * fator_sazonal).round();

    // Proteção de ruptura iminente (doc 02 §7.2): cobertura < N -> ao menos N dias base.
    if entrada.cobertura_dias < params.protecao_ruptura_dias as f64 {
        let piso = (entrada.media_diaria * params.dias_base_minimo as f64).round();
        qtd_final = qtd_final.max(piso);
    }
    (qtd_final as i64).max(0)
}

fn prioridade(entrada: &EntradaRecomendacao) -> PrioridadeProducao {
    if entrada.cobertura_dias < COBERTURA_URGENCIA_ALTA_DIAS || entrada.alerta_critico {
        PrioridadeProducao::Alta
    } else if entrada.cobertura_dias < COBERTURA_URGENCIA_MEDIA_DIAS
        || entrada.classe == ClasseAbc::A
    {
        PrioridadeProducao::Media
    } else {
        PrioridadeProducao::Baixa
    }
}

fn timing(cobertura_dias: f64, meta_dias: i64) -> Timing {
    let meta = meta_dias as f64;
    if cobertura_dias < meta * TIMING_IMEDIATO_FRACAO {
        Timing::Imediato
    } else if cobertura_dias < meta * TIMING_UMA_SEMANA_FRACAO {
        Timing::UmaSemana
    } else if cobertura_dias < meta {
        Timing::DuasSemanas
    } else if cobertura_dias > meta * TIMING_AGUARDAR_FRACAO {
        Timing::Aguardar
    } else {
        Timing::Monitorar
    }
}

#[cfg(test)]
mod testes {
    use super::{
        qtd_sugerida, recomendar_producao, EntradaRecomendacao, ParametrosRecomendacao,
        PrioridadeProducao, Timing,
    };
    use crate::tipos::ClasseAbc;

    fn params() -> ParametrosRecomendacao {
        ParametrosRecomendacao {
            meta_dias_classe: 45,
            fator_urgencia_lt7: 1.5,
            fator_urgencia_lt15: 1.2,
            fator_urgencia_default: 1.0,
            protecao_ruptura_dias: 3,
            dias_base_minimo: 15,
            lead_time_alta: 7,
            lead_time_media: 10,
            lead_time_baixa: 15,
        }
    }

    fn entrada(cobertura: f64, disponivel: i64) -> EntradaRecomendacao {
        EntradaRecomendacao {
            classe: ClasseAbc::A,
            media_diaria: 10.0,
            qtd_disponivel: disponivel,
            estoque_seguranca: 20,
            cobertura_dias: cobertura,
            fora_de_linha: false,
            alerta_critico: false,
        }
    }

    #[test]
    fn qtd_sugerida_simples() {
        assert_eq!(qtd_sugerida(100, 30, false, 10.0), 70);
        assert_eq!(qtd_sugerida(100, 120, false, 10.0), 0); // nunca negativo
        assert_eq!(qtd_sugerida(100, 30, true, 10.0), 0); // fora de linha
        assert_eq!(qtd_sugerida(100, 30, false, 0.0), 0); // sem histórico
    }

    #[test]
    fn quantidade_com_urgencia_e_prioridade() {
        // cobertura 5 (<7): urgência 1.5; ideal=450, necessária=450-50+20=420, final=630.
        let r = recomendar_producao(&entrada(5.0, 50), 1.0, &params());
        assert_eq!(r.qtd_final, 630);
        assert_eq!(r.prioridade, PrioridadeProducao::Alta);
        assert_eq!(r.lead_time_dias, 7);
        assert_eq!(r.timing, Timing::Imediato); // 5 < 45*0.3
    }

    #[test]
    fn protecao_de_ruptura_eleva_o_piso() {
        // cobertura 2 (<3): piso = round(10*15)=150 domina a quantidade calculada (10).
        let r = recomendar_producao(&entrada(2.0, 460), 1.0, &params());
        assert_eq!(r.qtd_final, 150);
    }

    #[test]
    fn fora_de_linha_nao_recomenda_quantidade() {
        let mut e = entrada(2.0, 0);
        e.fora_de_linha = true;
        assert_eq!(recomendar_producao(&e, 1.0, &params()).qtd_final, 0);
    }

    #[test]
    fn timing_escalona_por_cobertura() {
        let t =
            |cobertura: f64| recomendar_producao(&entrada(cobertura, 100), 1.0, &params()).timing;
        assert_eq!(t(10.0), Timing::Imediato); // < 45*0.3 = 13.5
        assert_eq!(t(20.0), Timing::UmaSemana); // < 45*0.6 = 27
        assert_eq!(t(40.0), Timing::DuasSemanas); // < 45
        assert_eq!(t(50.0), Timing::Monitorar); // entre 45 e 67.5
        assert_eq!(t(70.0), Timing::Aguardar); // > 45*1.5 = 67.5
    }

    #[test]
    fn qtd_final_nunca_negativa() {
        // disponível altíssimo -> necessária 0 -> final 0 (não negativo).
        let r = recomendar_producao(&entrada(30.0, 100_000), 1.0, &params());
        assert!(r.qtd_final >= 0);
    }
}
