//! Estatística da demanda diária (doc 02 §3.2/§3.3): percentis, IQR, média e desvio sem
//! outliers, coeficiente de variação. Funções puras.
// Casts entre contagens/quantidades (pequenas) e f64 são exatos (valores << 2^53).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

/// Uma venda diária datada. A DATA é necessária para dois motivos: densificar a série (dias sem
/// venda contam como zero) e pesar a recência.
#[derive(Debug, Clone, Copy)]
pub struct VendaDiaria {
    pub data: chrono::NaiveDate,
    pub qtd: i64,
}

/// Resumo estatístico da demanda diária, já com outliers removidos (doc 02 §3.2/§3.3).
pub(super) struct ResumoEstatistico {
    pub media: f64,
    pub desvio: f64,
    pub coef_variacao: f64,
    pub outliers: i64,
}

/// Média/desvio da demanda diária, sem outliers (IQR só no teto — doc 02 §3.2).
///
/// DUAS correções sobre a regra original (decisão do dono, 2026-07-13):
///
/// 1. **A média é por DIA CORRIDO**, não por "dia com venda". A regra antiga (doc 02 §3.1) dividia
///    pelos dias em que houve pedido — o que só faz sentido para um produto que vende todo dia.
///    Numa fábrica que vende em LOTES B2B, um item com 47 dias de venda no ano tinha a média
///    inflada ~8x, e como a cobertura é `disponível ÷ média`, tudo virava "Crítico" e o motor
///    mandava produzir múltiplos do necessário.
///
/// 2. **Os dias recentes pesam mais** (decaimento exponencial por `meia_vida_dias`). A janela de 12
///    meses tratava a venda de 12 meses atrás igual à de ontem, então um produto que MORREU seguia
///    pedindo produção com base no que vendia no auge. `meia_vida_dias = 0` desliga o decaimento.
///
/// Dias outlier saem da série inteira (não viram zero: são desconhecidos, não ausência de venda).
pub(super) fn resumo(
    vendas: &[VendaDiaria],
    data_ref: chrono::NaiveDate,
    janela_dias: i64,
    iqr_mult: f64,
    meia_vida_dias: f64,
) -> ResumoEstatistico {
    let mut valores: Vec<f64> = vendas
        .iter()
        .filter(|v| v.qtd > 0)
        .map(|v| v.qtd as f64)
        .collect();
    if valores.is_empty() || janela_dias <= 0 {
        return ResumoEstatistico {
            media: 0.0,
            desvio: 0.0,
            coef_variacao: 0.0,
            outliers: 0,
        };
    }
    valores.sort_by(f64::total_cmp);

    // Limite superior do IQR (doc 02 §3.2): Q3 + mult × (Q3 − Q1). Só o teto é aplicado.
    let q1 = percentil_cont(&valores, 0.25);
    let q3 = percentil_cont(&valores, 0.75);
    let limite_superior = q3 + iqr_mult * (q3 - q1);
    let outliers = valores.iter().filter(|&&v| v > limite_superior).count() as i64;

    // Série densificada: TODOS os dias corridos da janela, zero onde não houve venda.
    let mut por_dia: std::collections::HashMap<chrono::NaiveDate, f64> =
        std::collections::HashMap::with_capacity(vendas.len());
    for v in vendas {
        if v.qtd > 0 && (v.qtd as f64) <= limite_superior {
            *por_dia.entry(v.data).or_insert(0.0) += v.qtd as f64;
        }
    }
    let dias_outlier: std::collections::HashSet<chrono::NaiveDate> = vendas
        .iter()
        .filter(|v| (v.qtd as f64) > limite_superior)
        .map(|v| v.data)
        .collect();

    // Média/variância PONDERADAS pela recência sobre os dias corridos.
    let (mut soma_peso, mut soma_q, mut soma_q2) = (0.0_f64, 0.0_f64, 0.0_f64);
    for idade in 0..janela_dias {
        let Some(dia) = data_ref.checked_sub_signed(chrono::Duration::days(idade)) else {
            continue;
        };
        if dias_outlier.contains(&dia) {
            continue; // dia anômalo: fora da série (nem numerador, nem denominador)
        }
        let qtd = por_dia.get(&dia).copied().unwrap_or(0.0);
        let peso = if meia_vida_dias > 0.0 {
            0.5_f64.powf(idade as f64 / meia_vida_dias)
        } else {
            1.0
        };
        soma_peso += peso;
        soma_q += qtd * peso;
        soma_q2 += qtd * qtd * peso;
    }
    if soma_peso <= 0.0 {
        return ResumoEstatistico {
            media: 0.0,
            desvio: 0.0,
            coef_variacao: 0.0,
            outliers,
        };
    }
    let media = soma_q / soma_peso;
    let variancia = (soma_q2 / soma_peso - media * media).max(0.0);
    let desvio = variancia.sqrt();
    let coef_variacao = if media > 0.0 { desvio / media } else { 0.0 };

    ResumoEstatistico {
        media,
        desvio,
        coef_variacao,
        outliers,
    }
}

/// Percentil contínuo por interpolação linear (equivalente ao `percentile_cont` do Postgres).
/// `valores` deve estar ordenado de forma crescente e não pode ser vazio.
fn percentil_cont(valores: &[f64], p: f64) -> f64 {
    let n = valores.len();
    if n == 1 {
        return valores[0];
    }
    let rank = p * (n as f64 - 1.0);
    let inferior = rank.floor() as usize;
    let frac = rank - rank.floor();
    let base = valores[inferior];
    if inferior + 1 < n {
        base + frac * (valores[inferior + 1] - base)
    } else {
        base
    }
}

#[cfg(test)]
mod testes {
    use super::{percentil_cont, resumo, VendaDiaria};
    use chrono::NaiveDate;

    fn hoje() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 7, 13).unwrap()
    }

    /// Série de dias consecutivos terminando hoje.
    fn serie(qtds: &[i64]) -> Vec<VendaDiaria> {
        qtds.iter()
            .enumerate()
            .map(|(i, &qtd)| VendaDiaria {
                data: hoje() - chrono::Duration::days(i as i64),
                qtd,
            })
            .collect()
    }

    #[test]
    fn remove_outlier_superior_via_iqr() {
        // Nove 10 e um 100: Q1=Q3=10 -> IQR=0 -> limite=10; o 100 é outlier.
        let v = serie(&[10, 10, 10, 10, 10, 10, 10, 10, 10, 100]);
        let r = resumo(&v, hoje(), 365, 1.5, 0.0);
        assert_eq!(r.outliers, 1);
        // 9 dias x 10 = 90, sobre 364 dias corridos (o dia do outlier sai da série).
        assert!((r.media - 90.0 / 364.0).abs() < 1e-6, "media {}", r.media);
    }

    #[test]
    fn media_por_dia_corrido_dilui_a_venda_esparsa() {
        // 10 dias vendendo 10 -> 100 un no ano -> 100/365 por dia corrido (NÃO 10/dia).
        let v = serie(&[10; 10]);
        let r = resumo(&v, hoje(), 365, 1.5, 0.0);
        assert_eq!(r.outliers, 0);
        assert!((r.media - 100.0 / 365.0).abs() < 1e-6, "media {}", r.media);
    }

    #[test]
    fn recencia_pesa_mais_que_o_passado() {
        // Mesma quantidade, mas uma série vendeu ONTEM e a outra há ~1 ano.
        let recente = vec![VendaDiaria {
            data: hoje(),
            qtd: 100,
        }];
        let antiga = vec![VendaDiaria {
            data: hoje() - chrono::Duration::days(360),
            qtd: 100,
        }];
        let r_rec = resumo(&recente, hoje(), 365, 1.5, 90.0);
        let r_ant = resumo(&antiga, hoje(), 365, 1.5, 90.0);
        assert!(
            r_rec.media > r_ant.media * 10.0,
            "venda de ontem deve pesar muito mais: {} vs {}",
            r_rec.media,
            r_ant.media
        );
    }

    #[test]
    fn percentis_batem_com_percentile_cont() {
        // percentile_cont sobre 1..=10: Q1 = 3.25, Q3 = 7.75.
        let v: Vec<f64> = (1..=10).map(f64::from).collect();
        assert!((percentil_cont(&v, 0.25) - 3.25).abs() < 1e-9);
        assert!((percentil_cont(&v, 0.75) - 7.75).abs() < 1e-9);
    }
}
