//! Insights estatísticos por produto (doc 06 §3): decomposição sazonal por dia da semana,
//! previsão 7/30d (§3.2) e alertas inteligentes (§3.3). Motor local PURO e testável (§11) — sem
//! correlações mockadas do legado (§3.5). A IA generativa (chat/análise) é à parte (4.2/4.3).
#![allow(clippy::cast_precision_loss)] // séries pequenas (≤365): casts exatos

use chrono::{Datelike, Duration, NaiveDate};
use serde::Serialize;

use crate::estatistica::{
    limpar_outliers_iqr, media_movel, projetar, regressao_linear, suavizacao_exponencial,
};

/// Um ponto da série diária de vendas (dia → quantidade).
#[derive(Debug, Clone, Copy)]
pub struct PontoVenda {
    pub data: NaiveDate,
    pub qtd: f64,
}

/// Contexto do produto (vindo do motor/`produto_ativo`) para os alertas de §3.3.
#[derive(Debug, Clone, Copy)]
pub struct ContextoProduto {
    pub cobertura_dias: f64,
    pub qtd_disponivel: i64,
    pub estoque_recomendado: i64,
}

/// Alerta inteligente (doc 06 §3.3). Categorias e severidades canônicas do doc.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct AlertaInteligente {
    pub categoria: String,
    pub severidade: String,
    pub titulo: String,
    pub detalhe: String,
}

/// Resultado dos insights de um produto.
#[derive(Debug, Clone, Serialize)]
pub struct Insights {
    pub slope: f64,
    pub correlacao: f64,
    pub baseline_ma7: f64,
    pub forca_sazonal: f64,
    pub fatores_dia: [f64; 7],
    pub previsao_7d: Vec<f64>,
    pub total_previsto_7d: f64,
    pub total_previsto_30d: f64,
    pub confianca: f64,
    pub dias_com_venda_pct: f64,
    pub alertas: Vec<AlertaInteligente>,
}

const ALFA_SUAVIZACAO: f64 = 0.3;
const HORIZONTE_LONGO: usize = 30;

/// Calcula os insights de um produto a partir da série de vendas no intervalo `[inicio, fim]`.
/// A série é densificada (dias sem venda = 0) para a sazonalidade por dia da semana ser correta.
#[must_use]
pub fn analisar(
    pontos: &[PontoVenda],
    inicio: NaiveDate,
    fim: NaiveDate,
    ctx: &ContextoProduto,
) -> Insights {
    let serie = densificar(pontos, inicio, fim);
    let dias = serie.len();
    let limpa = limpar_outliers_iqr(&serie, 1.5);
    let tendencia = regressao_linear(&limpa);
    let ma7 = media_movel(&limpa, 7);
    let suave = suavizacao_exponencial(&limpa, ALFA_SUAVIZACAO);
    let baseline_ma7 = ma7.last().copied().unwrap_or(0.0);
    let ultimo_suave = suave.last().copied().unwrap_or(0.0);

    let (fatores_dia, forca_sazonal) = sazonalidade(inicio, &serie);

    // Previsão dos próximos HORIZONTE_LONGO dias (doc 06 §3.2).
    let mut previsao = Vec::with_capacity(HORIZONTE_LONGO);
    for h in 1..=HORIZONTE_LONGO {
        let dia = fim + Duration::days(i64::try_from(h).unwrap_or(0));
        let idx_dia = dia.weekday().num_days_from_monday() as usize;
        let tendencia_h = projetar(&tendencia, dias + h - 1);
        let p = tendencia_h * (0.4 * tendencia.correlacao.abs())
            + ultimo_suave * fatores_dia[idx_dia] * (0.3 * forca_sazonal)
            + baseline_ma7 * 0.3;
        previsao.push(p.max(0.0));
    }
    let previsao_7d: Vec<f64> = previsao.iter().take(7).copied().collect();
    let total_previsto_7d: f64 = previsao_7d.iter().sum();
    let total_previsto_30d: f64 = previsao.iter().sum();

    let confianca = f64::midpoint(tendencia.correlacao.abs(), forca_sazonal).min(0.95);

    let dias_com_venda_pct = if dias == 0 {
        0.0
    } else {
        serie.iter().filter(|&&v| v > 0.0).count() as f64 / dias as f64 * 100.0
    };
    let media_historica = if dias == 0 {
        0.0
    } else {
        serie.iter().sum::<f64>() / dias as f64
    };

    let alertas = gerar_alertas(
        ctx,
        &previsao_7d,
        total_previsto_7d,
        media_historica,
        forca_sazonal,
        tendencia.slope,
        dias_com_venda_pct,
    );

    Insights {
        slope: tendencia.slope,
        correlacao: tendencia.correlacao,
        baseline_ma7,
        forca_sazonal,
        fatores_dia,
        previsao_7d,
        total_previsto_7d,
        total_previsto_30d,
        confianca,
        dias_com_venda_pct,
        alertas,
    }
}

/// Série diária densa em `[inicio, fim]` (dias sem registro = 0); soma quantidades do mesmo dia.
fn densificar(pontos: &[PontoVenda], inicio: NaiveDate, fim: NaiveDate) -> Vec<f64> {
    if fim < inicio {
        return Vec::new();
    }
    let total = (fim - inicio).num_days() + 1;
    let mut serie = vec![0.0; usize::try_from(total).unwrap_or(0)];
    for p in pontos {
        if p.data >= inicio && p.data <= fim {
            let idx = usize::try_from((p.data - inicio).num_days()).unwrap_or(0);
            serie[idx] += p.qtd;
        }
    }
    serie
}

/// Decomposição sazonal por dia da semana (doc 06 §3.1): fator por dia (média do dia / média
/// geral) e força sazonal = dispersão dos fatores, limitada a `[0, 1]`.
fn sazonalidade(inicio: NaiveDate, serie: &[f64]) -> ([f64; 7], f64) {
    let mut soma = [0.0_f64; 7];
    let mut cont = [0_u32; 7];
    for (i, &v) in serie.iter().enumerate() {
        let dia = (inicio + Duration::days(i64::try_from(i).unwrap_or(0)))
            .weekday()
            .num_days_from_monday() as usize;
        soma[dia] += v;
        cont[dia] += 1;
    }
    let media_geral = if serie.is_empty() {
        0.0
    } else {
        serie.iter().sum::<f64>() / serie.len() as f64
    };
    let mut fatores = [1.0_f64; 7];
    if media_geral > 0.0 {
        for d in 0..7 {
            if cont[d] > 0 {
                fatores[d] = (soma[d] / f64::from(cont[d])) / media_geral;
            }
        }
    }
    // Força = desvio-padrão dos fatores em torno de 1.0, limitado a [0,1].
    let var = fatores.iter().map(|f| (f - 1.0).powi(2)).sum::<f64>() / 7.0;
    (fatores, var.sqrt().min(1.0))
}

/// Alertas inteligentes do doc 06 §3.3.
fn gerar_alertas(
    ctx: &ContextoProduto,
    previsao_7d: &[f64],
    total_previsto_7d: f64,
    media_historica: f64,
    forca_sazonal: f64,
    slope: f64,
    dias_com_venda_pct: f64,
) -> Vec<AlertaInteligente> {
    let mut alertas = Vec::new();
    let novo =
        |categoria: &str, severidade: &str, titulo: &str, detalhe: String| AlertaInteligente {
            categoria: categoria.to_owned(),
            severidade: severidade.to_owned(),
            titulo: titulo.to_owned(),
            detalhe,
        };

    if ctx.cobertura_dias < 7.0 {
        alertas.push(novo(
            "ruptura",
            "critico",
            "Ruptura crítica",
            format!("Cobertura de {:.1} dias (< 7).", ctx.cobertura_dias),
        ));
    }

    if ctx.estoque_recomendado > 0 {
        let deficit = (ctx.estoque_recomendado - ctx.qtd_disponivel) as f64
            / ctx.estoque_recomendado as f64
            * 100.0;
        if deficit > 30.0 {
            let sev = if deficit > 70.0 { "critico" } else { "atencao" };
            alertas.push(novo(
                "ruptura",
                sev,
                "Meta ABC não atingida",
                format!("Déficit de {deficit:.0}% do estoque recomendado."),
            ));
        }
    }

    if total_previsto_7d > ctx.qtd_disponivel as f64 {
        alertas.push(novo(
            "ruptura",
            "atencao",
            "Ruptura do disponível em 7 dias",
            format!(
                "Previsão de {total_previsto_7d:.0} un > {} disponíveis.",
                ctx.qtd_disponivel
            ),
        ));
    }

    if media_historica > 0.0 {
        let media_prev = total_previsto_7d / 7.0;
        let variacao = (media_prev - media_historica) / media_historica * 100.0;
        if variacao > 20.0 {
            alertas.push(novo(
                "demanda",
                "positivo",
                "Demanda em alta",
                format!("Previsão {variacao:.0}% acima da média histórica."),
            ));
        } else if variacao < -20.0 {
            alertas.push(novo(
                "demanda",
                "informativo",
                "Demanda em queda",
                format!("Previsão {variacao:.0}% abaixo da média histórica."),
            ));
        }
    }

    let media_prev = if previsao_7d.is_empty() {
        0.0
    } else {
        total_previsto_7d / previsao_7d.len() as f64
    };
    if forca_sazonal > 0.5 && previsao_7d.iter().any(|&d| d > media_prev * 1.3) {
        alertas.push(novo(
            "sazonalidade",
            "informativo",
            "Pico sazonal próximo",
            "Há dias na semana com demanda prevista bem acima da média.".to_owned(),
        ));
    }

    if ctx.cobertura_dias > 90.0 && slope < 0.0 {
        alertas.push(novo(
            "excesso",
            "atencao",
            "Excesso de estoque",
            format!(
                "Cobertura de {:.0} dias (> 3 meses) com tendência de queda.",
                ctx.cobertura_dias
            ),
        ));
    }

    if dias_com_venda_pct < 50.0 {
        alertas.push(novo(
            "padrao",
            "informativo",
            "Qualidade de dados",
            format!("Apenas {dias_com_venda_pct:.0}% dos dias com venda."),
        ));
    }

    alertas
}

#[cfg(test)]
mod testes {
    use super::{analisar, ContextoProduto, PontoVenda};
    use chrono::NaiveDate;

    fn data(ymd: (i32, u32, u32)) -> NaiveDate {
        NaiveDate::from_ymd_opt(ymd.0, ymd.1, ymd.2).unwrap()
    }

    #[test]
    fn ruptura_critica_por_cobertura() {
        let inicio = data((2026, 1, 1));
        let fim = data((2026, 3, 31));
        let pontos: Vec<PontoVenda> = Vec::new(); // sem vendas
        let ctx = ContextoProduto {
            cobertura_dias: 3.0,
            qtd_disponivel: 10,
            estoque_recomendado: 100,
        };
        let r = analisar(&pontos, inicio, fim, &ctx);
        assert!(r
            .alertas
            .iter()
            .any(|a| a.titulo == "Ruptura crítica" && a.severidade == "critico"));
        // Sem vendas → 0% dos dias com venda → alerta de qualidade de dados.
        assert!(r.alertas.iter().any(|a| a.categoria == "padrao"));
    }

    #[test]
    fn tendencia_de_alta_eleva_previsao() {
        let inicio = data((2026, 1, 1));
        let fim = data((2026, 3, 31));
        // Vendas crescentes diárias.
        let mut pontos = Vec::new();
        let mut d = inicio;
        let mut q = 1.0;
        while d <= fim {
            pontos.push(PontoVenda { data: d, qtd: q });
            q += 0.5;
            d = d.succ_opt().unwrap();
        }
        let ctx = ContextoProduto {
            cobertura_dias: 30.0,
            qtd_disponivel: 1000,
            estoque_recomendado: 500,
        };
        let r = analisar(&pontos, inicio, fim, &ctx);
        assert!(r.slope > 0.0);
        assert!(r.correlacao > 0.9);
        assert!(r.total_previsto_7d > 0.0);
        assert!(r.confianca > 0.0 && r.confianca <= 0.95);
        assert_eq!(r.previsao_7d.len(), 7);
    }
}
