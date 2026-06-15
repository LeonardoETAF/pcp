//! Estatística da demanda diária (doc 02 §3.2/§3.3): percentis, IQR, média e desvio sem
//! outliers, coeficiente de variação. Funções puras.
// Casts entre contagens/quantidades (pequenas) e f64 são exatos (valores << 2^53).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

/// Resumo estatístico da demanda diária, já com outliers removidos (doc 02 §3.2/§3.3).
pub(super) struct ResumoEstatistico {
    pub media: f64,
    pub desvio: f64,
    pub coef_variacao: f64,
    pub outliers: i64,
}

/// Média/desvio sem outliers (IQR só no limite superior) + coef. de variação.
/// Considera apenas dias com venda (`qtd > 0`), conforme doc 02 §3.1.
pub(super) fn resumo(vendas_diarias: &[i64], iqr_mult: f64) -> ResumoEstatistico {
    let mut valores: Vec<f64> = vendas_diarias
        .iter()
        .filter(|&&qtd| qtd > 0)
        .map(|&qtd| qtd as f64)
        .collect();
    if valores.is_empty() {
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

    let sem_outliers: Vec<f64> = valores
        .iter()
        .copied()
        .filter(|&v| v <= limite_superior)
        .collect();
    let outliers = (valores.len() - sem_outliers.len()) as i64;
    let media = media(&sem_outliers);
    let desvio = desvio_amostral(&sem_outliers, media);
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

fn media(valores: &[f64]) -> f64 {
    if valores.is_empty() {
        return 0.0;
    }
    valores.iter().sum::<f64>() / valores.len() as f64
}

/// Desvio padrão amostral (denominador n−1, como o `STDDEV` do Postgres). 0 para n < 2.
fn desvio_amostral(valores: &[f64], media: f64) -> f64 {
    let n = valores.len();
    if n < 2 {
        return 0.0;
    }
    let soma_quadrados: f64 = valores.iter().map(|v| (v - media).powi(2)).sum();
    (soma_quadrados / (n as f64 - 1.0)).sqrt()
}

#[cfg(test)]
mod testes {
    use super::{percentil_cont, resumo};

    #[test]
    fn remove_outlier_superior_via_iqr() {
        // Nove 10 e um 100: Q1=Q3=10 -> IQR=0 -> limite=10; o 100 é outlier.
        let v = [10, 10, 10, 10, 10, 10, 10, 10, 10, 100];
        let r = resumo(&v, 1.5);
        assert!((r.media - 10.0).abs() < 1e-9);
        assert_eq!(r.outliers, 1);
        assert!(r.desvio.abs() < 1e-9);
        assert!(r.coef_variacao.abs() < 1e-9);
    }

    #[test]
    fn sem_outliers_usa_todos_os_valores() {
        let v = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let r = resumo(&v, 1.5);
        assert_eq!(r.outliers, 0);
        assert!((r.media - 5.5).abs() < 1e-9);
    }

    #[test]
    fn ignora_dias_sem_venda() {
        // Zeros são ignorados: média sobre os dias COM venda (doc 02 §3.1).
        let v = [0, 10, 0, 10];
        let r = resumo(&v, 1.5);
        assert!((r.media - 10.0).abs() < 1e-9);
    }

    #[test]
    fn percentis_batem_com_percentile_cont() {
        // percentile_cont sobre 1..=10: Q1 = 3.25, Q3 = 7.75.
        let v: Vec<f64> = (1..=10).map(f64::from).collect();
        assert!((percentil_cont(&v, 0.25) - 3.25).abs() < 1e-9);
        assert!((percentil_cont(&v, 0.75) - 7.75).abs() < 1e-9);
    }
}
