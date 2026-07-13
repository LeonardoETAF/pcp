//! Cálculo dos parâmetros de estoque de um produto (doc 02 §3): aplica a estatística e a
//! fórmula unificada, ou usa os defaults quando o histórico é insuficiente (§3.4).

use chrono::NaiveDate;

use super::estatistica::{self, VendaDiaria};
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
    /// Dias corridos da janela de vendas (12 meses = 365). A média divide por ISTO, não pelos
    /// dias com venda (correção da §3.1 — decisão do dono, 2026-07-13).
    pub janela_dias: i64,
    /// Meia-vida do decaimento por recência, em dias. 0 desliga (todos os dias pesam igual).
    pub meia_vida_dias: f64,
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
    /// Média diária (dias corridos) do MÊS SEGUINTE no ano passado — o que este produto vendia
    /// nesta mesma época, um ano atrás. `None` quando o produto ainda não existia lá (não é
    /// ausência de demanda; é ausência de produto). Decisão do dono, 2026-07-13.
    pub demanda_mes_seguinte: Option<f64>,
}

/// Calcula os parâmetros de estoque de um produto (doc 02 §3).
///
/// - `vendas_diarias`: quantidades diárias consolidadas na janela de 12 meses (a filtragem
///   de janela é do chamador — agregação no banco, §15; aqui só dias com venda contam).
/// - `meta_dias_classe`: meta de cobertura da classe vigente (doc 02 §3.6).
/// - `fator_sazonal`: fator do mês da `data_ref` (doc 02 §4).
#[must_use]
pub fn calcular_parametros(
    vendas: &[VendaDiaria],
    data_ref: NaiveDate,
    meta_dias_classe: i64,
    fator_sazonal: f64,
    demanda_mes_seguinte: Option<f64>,
    config: &ParametrosEstoqueConfig,
) -> ParametrosEstoque {
    let dias_com_vendas =
        i64::try_from(vendas.iter().filter(|v| v.qtd > 0).count()).unwrap_or(i64::MAX);

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
            demanda_mes_seguinte,
        };
    }

    let resumo = estatistica::resumo(
        vendas,
        data_ref,
        config.janela_dias,
        config.outlier_iqr_mult,
        config.meia_vida_dias,
    );
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
        demanda_mes_seguinte,
    }
}

#[cfg(test)]
mod testes {
    use super::{
        calcular_parametros, DefaultsSemHistorico, ParametrosEstoqueConfig, StatusParametros,
        VendaDiaria,
    };
    use chrono::NaiveDate;

    const HOJE: (i32, u32, u32) = (2026, 7, 13);

    fn data_ref() -> NaiveDate {
        NaiveDate::from_ymd_opt(HOJE.0, HOJE.1, HOJE.2).unwrap()
    }

    /// Série com `n` dias de venda, um por dia, terminando em `data_ref`.
    fn dias_seguidos(n: i64, qtd: i64) -> Vec<VendaDiaria> {
        (0..n)
            .map(|i| VendaDiaria {
                data: data_ref() - chrono::Duration::days(i),
                qtd,
            })
            .collect()
    }

    fn config() -> ParametrosEstoqueConfig {
        ParametrosEstoqueConfig {
            min_dias_com_vendas: 10,
            outlier_iqr_mult: 1.5,
            z_score_seguranca: 1.28,
            teto_cobertura_dias: 60,
            fracao_minimo: 0.70,
            janela_dias: 365,
            meia_vida_dias: 0.0, // sem decaimento, para isolar o efeito nos testes
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
        let p = calcular_parametros(&dias_seguidos(5, 10), data_ref(), 45, 1.0, None, &config());
        assert_eq!(p.status, StatusParametros::SemHistoricoConfiavel);
        assert_eq!(p.dias_com_vendas, 5);
        assert_eq!(p.estoque_total_recomendado, 1000);
    }

    /// A CORREÇÃO central: a média é por dia CORRIDO, não por dia com venda.
    /// 30 dias vendendo 100 numa janela de 365 → 3000/365 ≈ 8,2/dia (não 100/dia).
    #[test]
    fn media_e_por_dia_corrido_nao_por_dia_com_venda() {
        let p = calcular_parametros(
            &dias_seguidos(30, 100),
            data_ref(),
            45,
            1.0,
            None,
            &config(),
        );
        assert_eq!(p.status, StatusParametros::Calculado);
        assert_eq!(p.dias_com_vendas, 30);
        let esperado = 3000.0 / 365.0;
        assert!(
            (p.media_diaria - esperado).abs() < 0.1,
            "média deve ser {esperado:.2}/dia (dias corridos), veio {:.2}",
            p.media_diaria
        );
    }

    /// Produto que MORREU: vendia forte há 10 meses, nada nos últimos 3. Com decaimento, a média
    /// desaba — sem ele, o auge do passado seguiria mandando produzir.
    #[test]
    fn decaimento_faz_produto_morto_perder_a_media() {
        // 30 dias de venda alta, há ~300 dias; nada desde então.
        let vendas: Vec<VendaDiaria> = (300..330)
            .map(|i| VendaDiaria {
                data: data_ref() - chrono::Duration::days(i),
                qtd: 1000,
            })
            .collect();

        let sem_decaimento = calcular_parametros(&vendas, data_ref(), 45, 1.0, None, &config());
        let mut cfg = config();
        cfg.meia_vida_dias = 90.0;
        let com_decaimento = calcular_parametros(&vendas, data_ref(), 45, 1.0, None, &cfg);

        // Sem decaimento: 30 x 1000 / 365 = 82,2/dia — o auge de 10 meses atrás ainda manda.
        // Com meia-vida de 90d: aquelas vendas pesam ~0,09, e a média cai para ~21,8/dia.
        assert!(
            com_decaimento.media_diaria < sem_decaimento.media_diaria / 3.0,
            "o decaimento deve derrubar a média do produto morto: {:.2} vs {:.2}",
            com_decaimento.media_diaria,
            sem_decaimento.media_diaria
        );
    }

    #[test]
    fn outlier_nao_infla_a_media() {
        let mut vendas = dias_seguidos(11, 10);
        vendas.push(VendaDiaria {
            data: data_ref() - chrono::Duration::days(20),
            qtd: 1000,
        });
        let p = calcular_parametros(&vendas, data_ref(), 45, 1.0, None, &config());
        assert_eq!(p.outliers_detectados, 1);
        // 11 dias x 10 = 110, sobre ~364 dias corridos (o dia outlier sai da série).
        assert!(p.media_diaria < 1.0, "média baixa: {:.3}", p.media_diaria);
    }

    #[test]
    fn demanda_do_mes_seguinte_atravessa_o_calculo() {
        let p = calcular_parametros(
            &dias_seguidos(30, 100),
            data_ref(),
            45,
            1.0,
            Some(42.0),
            &config(),
        );
        assert_eq!(p.demanda_mes_seguinte, Some(42.0));
    }
}
