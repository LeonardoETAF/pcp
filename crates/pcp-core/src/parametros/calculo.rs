//! Cálculo dos parâmetros de estoque de um produto (doc 02 §3): aplica a estatística e a
//! fórmula unificada, ou usa os defaults quando o histórico é insuficiente (§3.4).

use super::estatistica;
use super::formula;

/// Qualidade do histórico do produto (doc 02 §3.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusParametros {
    /// Histórico suficiente (`dias_com_vendas >= min_dias_com_vendas`): parâmetros calculados.
    Calculado,
    /// Histórico insuficiente: usa defaults configuráveis (doc 02 §3.4).
    SemHistoricoConfiavel,
}

/// Valores default para produtos sem histórico confiável (doc 02 §3.4).
#[derive(Debug, Clone, Copy)]
pub struct DefaultsSemHistorico {
    pub media: f64,
    pub minimo: i64,
    pub seguranca: i64,
    pub recomendado: i64,
}

/// Limiares dos parâmetros de estoque (doc 02 §3/§11). Originados de pcp-config (§2/§13).
#[derive(Debug, Clone, Copy)]
pub struct ParametrosEstoqueConfig {
    /// Mínimo de dias com venda para calcular (abaixo disso, `SEM_HISTORICO_CONFIAVEL`).
    pub min_dias_com_vendas: i64,
    /// Multiplicador do IQR para o limite superior de outliers (doc 02 §3.2: 1.5).
    pub outlier_iqr_mult: f64,
    /// Z-score do estoque de segurança (doc 02 §3.5: 1.28 = 90%).
    pub z_score_seguranca: f64,
    /// Teto de cobertura, em dias, do estoque recomendado (doc 02 §3.5: 60).
    pub teto_cobertura_dias: i64,
    /// Fração do alvo-meta que define o estoque mínimo (doc 02 §3.6: 0.70).
    pub fracao_minimo: f64,
    /// Defaults para histórico insuficiente (doc 02 §3.4).
    pub defaults_sem_historico: DefaultsSemHistorico,
}

/// Parâmetros estatísticos calculados de um produto (doc 02 §3 / doc 04 §3.2).
#[derive(Debug, Clone, PartialEq)]
pub struct ParametrosEstoque {
    pub status: StatusParametros,
    pub media_diaria: f64,
    pub desvio: f64,
    pub coef_variacao: f64,
    pub dias_com_vendas: i64,
    pub outliers_detectados: i64,
    pub estoque_minimo: i64,
    pub estoque_seguranca: i64,
    pub estoque_total_recomendado: i64,
}

/// Calcula os parâmetros de estoque de um produto (doc 02 §3).
///
/// - `vendas_diarias`: quantidades diárias consolidadas na janela de 12 meses (a filtragem
///   de janela é do chamador — agregação no banco, §15; aqui só dias com venda contam).
/// - `meta_dias_classe`: meta de cobertura da classe vigente (doc 02 §3.6).
/// - `fator_sazonal`: fator do mês da `data_ref` (doc 02 §4).
#[must_use]
pub fn calcular_parametros(
    vendas_diarias: &[i64],
    meta_dias_classe: i64,
    fator_sazonal: f64,
    config: &ParametrosEstoqueConfig,
) -> ParametrosEstoque {
    let dias_com_vendas =
        i64::try_from(vendas_diarias.iter().filter(|&&qtd| qtd > 0).count()).unwrap_or(i64::MAX);

    if dias_com_vendas < config.min_dias_com_vendas {
        let padrao = config.defaults_sem_historico;
        return ParametrosEstoque {
            status: StatusParametros::SemHistoricoConfiavel,
            media_diaria: padrao.media,
            desvio: 0.0,
            coef_variacao: 0.0,
            dias_com_vendas,
            outliers_detectados: 0,
            estoque_minimo: padrao.minimo,
            estoque_seguranca: padrao.seguranca,
            estoque_total_recomendado: padrao.recomendado,
        };
    }

    let resumo = estatistica::resumo(vendas_diarias, config.outlier_iqr_mult);
    let recomendacao = formula::recomendar(
        resumo.media,
        resumo.desvio,
        meta_dias_classe,
        fator_sazonal,
        config,
    );
    ParametrosEstoque {
        status: StatusParametros::Calculado,
        media_diaria: resumo.media,
        desvio: resumo.desvio,
        coef_variacao: resumo.coef_variacao,
        dias_com_vendas,
        outliers_detectados: resumo.outliers,
        estoque_minimo: recomendacao.minimo,
        estoque_seguranca: recomendacao.seguranca,
        estoque_total_recomendado: recomendacao.total_recomendado,
    }
}

#[cfg(test)]
mod testes {
    use super::{
        calcular_parametros, DefaultsSemHistorico, ParametrosEstoqueConfig, StatusParametros,
    };

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
    fn sem_historico_confiavel_usa_defaults() {
        // 5 dias com venda (< 10) -> defaults (doc 02 §3.4).
        let p = calcular_parametros(&[10, 10, 10, 10, 10], 45, 1.0, &config());
        assert_eq!(p.status, StatusParametros::SemHistoricoConfiavel);
        assert!((p.media_diaria - 50.0).abs() < 1e-9);
        assert_eq!(p.dias_com_vendas, 5);
        assert_eq!(p.estoque_minimo, 750);
        assert_eq!(p.estoque_seguranca, 250);
        assert_eq!(p.estoque_total_recomendado, 1000);
    }

    #[test]
    fn calculado_com_historico_suficiente() {
        // 12 dias, todos 10 (desvio 0); classe A (45d), sazonal 1.0.
        let dias = [10_i64; 12];
        let p = calcular_parametros(&dias, 45, 1.0, &config());
        assert_eq!(p.status, StatusParametros::Calculado);
        assert!((p.media_diaria - 10.0).abs() < 1e-9);
        assert_eq!(p.dias_com_vendas, 12);
        assert_eq!(p.estoque_seguranca, 0); // desvio 0
        assert_eq!(p.estoque_total_recomendado, 450); // round(10*45) + 0, abaixo do teto 600
        assert_eq!(p.estoque_minimo, 315); // round(450 * 0.70)
    }

    #[test]
    fn outlier_nao_infla_a_media() {
        // 11 dias em 10 + um pico de 1000: o pico é removido (IQR), média continua 10.
        let mut dias = vec![10_i64; 11];
        dias.push(1000);
        let p = calcular_parametros(&dias, 45, 1.0, &config());
        assert_eq!(p.status, StatusParametros::Calculado);
        assert!((p.media_diaria - 10.0).abs() < 1e-9);
        assert_eq!(p.outliers_detectados, 1);
    }
}
