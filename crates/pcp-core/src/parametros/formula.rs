//! Estoque recomendado UNIFICADO (CLAUDE.md §4 / doc 02 §3.6 + segurança/teto da §3.5).
// Quantidades resultantes são pequenas e não-negativas; os casts f64<->i64 são seguros.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use super::calculo::ParametrosEstoqueConfig;

/// Resultado da recomendação de estoque (unidades inteiras).
pub(super) struct Recomendacao {
    pub minimo: i64,
    pub seguranca: i64,
    pub total_recomendado: i64,
}

/// Calcula o estoque recomendado pela fórmula meta-ABC unificada (doc 02 §3.6):
/// - `alvo` = ROUND(media × `meta_dias` × sazonal);
/// - `seguranca` = CEIL(desvio × z × sazonal)   (z de 90% — §3.5);
/// - `recomendado` = MIN(alvo + seguranca, teto)  com teto = CEIL(media × `teto_cobertura_dias`);
/// - `minimo` = ROUND(alvo × `fracao_minimo`)       (70% do alvo — §3.6).
pub(super) fn recomendar(
    media: f64,
    desvio: f64,
    meta_dias: i64,
    fator_sazonal: f64,
    config: &ParametrosEstoqueConfig,
) -> Recomendacao {
    let alvo = (media * meta_dias as f64 * fator_sazonal).round();
    let seguranca = (desvio * config.z_score_seguranca * fator_sazonal).ceil();
    let teto = (media * config.teto_cobertura_dias as f64).ceil();
    let total = (alvo + seguranca).min(teto);
    let minimo = (alvo * config.fracao_minimo).round();
    Recomendacao {
        minimo: minimo as i64,
        seguranca: seguranca as i64,
        total_recomendado: total as i64,
    }
}

#[cfg(test)]
mod testes {
    use super::recomendar;
    use crate::parametros::calculo::{DefaultsSemHistorico, ParametrosEstoqueConfig};

    fn config() -> ParametrosEstoqueConfig {
        ParametrosEstoqueConfig {
            min_dias_com_vendas: 10,
            outlier_iqr_mult: 1.5,
            z_score_seguranca: 1.28,
            teto_cobertura_dias: 60,
            fracao_minimo: 0.70,
            defaults_sem_historico: DefaultsSemHistorico {
                media: 50.0,
                minimo: 750,
                seguranca: 250,
                recomendado: 1000,
            },
        }
    }

    #[test]
    fn recomendacao_unificada_meta_abc() {
        // media 10, desvio 2, classe A (45d), sazonal 1.0:
        // alvo=450, seguranca=ceil(2*1.28)=3, recomendado=min(453,600)=453, minimo=round(315)=315.
        let r = recomendar(10.0, 2.0, 45, 1.0, &config());
        assert_eq!(r.total_recomendado, 453);
        assert_eq!(r.seguranca, 3);
        assert_eq!(r.minimo, 315);
    }

    #[test]
    fn teto_de_60_dias_limita_o_recomendado() {
        // sazonal 2.0 inflaria o alvo (900), mas o teto = ceil(10*60) = 600 limita.
        let r = recomendar(10.0, 2.0, 45, 2.0, &config());
        assert_eq!(r.total_recomendado, 600);
    }
}
