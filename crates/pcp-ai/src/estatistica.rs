//! Primitivas estatísticas dos insights (doc 06 §3.1). Funções PURAS sobre séries de `f64`,
//! testáveis (CLAUDE.md §11) — substituem o `mlAlgorithms.ts` do legado, que rodava no browser.
// Séries pequenas (≤365 pontos) e índices não-negativos: os casts f64<->usize são exatos.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

/// Tendência por regressão linear simples (índice → valor) + correlação de Pearson (doc 06 §3.1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tendencia {
    pub slope: f64,
    pub intercepto: f64,
    pub correlacao: f64,
}

/// Regressão linear de `y` sobre o índice `0..n`. Série < 2 pontos → tudo zero.
#[must_use]
pub fn regressao_linear(y: &[f64]) -> Tendencia {
    let n = y.len();
    if n < 2 {
        return Tendencia {
            slope: 0.0,
            intercepto: y.first().copied().unwrap_or(0.0),
            correlacao: 0.0,
        };
    }
    let n_f = n as f64;
    let media_x = (n_f - 1.0) / 2.0;
    let media_y = y.iter().sum::<f64>() / n_f;
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (i, &yi) in y.iter().enumerate() {
        let dx = i as f64 - media_x;
        let dy = yi - media_y;
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    let slope = if sxx > 0.0 { sxy / sxx } else { 0.0 };
    let correlacao = if sxx > 0.0 && syy > 0.0 {
        sxy / (sxx.sqrt() * syy.sqrt())
    } else {
        0.0
    };
    Tendencia {
        slope,
        intercepto: media_y - slope * media_x,
        correlacao,
    }
}

/// Valor projetado da tendência no índice `i` (pode ser futuro).
#[must_use]
pub fn projetar(t: &Tendencia, i: usize) -> f64 {
    (t.slope * i as f64 + t.intercepto).max(0.0)
}

/// Média móvel de janela `j` (doc 06 §3.1). Cada ponto = média dos até `j` valores anteriores.
#[must_use]
pub fn media_movel(y: &[f64], j: usize) -> Vec<f64> {
    if j == 0 {
        return y.to_vec();
    }
    (0..y.len())
        .map(|i| {
            let inicio = i.saturating_sub(j - 1);
            let janela = &y[inicio..=i];
            janela.iter().sum::<f64>() / janela.len() as f64
        })
        .collect()
}

/// Suavização exponencial simples com fator `alfa` (doc 06 §3.1).
#[must_use]
pub fn suavizacao_exponencial(y: &[f64], alfa: f64) -> Vec<f64> {
    let mut saida = Vec::with_capacity(y.len());
    let Some(&primeiro) = y.first() else {
        return saida;
    };
    let mut s = primeiro;
    for &v in y {
        s = alfa * v + (1.0 - alfa) * s;
        saida.push(s);
    }
    saida
}

/// Quartis (Q1, Q3) por interpolação linear sobre a série ordenada.
fn quartis(y: &[f64]) -> Option<(f64, f64)> {
    if y.len() < 4 {
        return None;
    }
    let mut ord = y.to_vec();
    ord.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let percentil = |p: f64| {
        let pos = p * (ord.len() as f64 - 1.0);
        let baixo = pos.floor() as usize;
        let alto = pos.ceil() as usize;
        let frac = pos - baixo as f64;
        ord[baixo] + (ord[alto] - ord[baixo]) * frac
    };
    Some((percentil(0.25), percentil(0.75)))
}

/// Remove outliers IQR (1.5× por padrão) por *clamp* aos limites (doc 06 §3.1 — limpeza da série).
#[must_use]
pub fn limpar_outliers_iqr(y: &[f64], mult: f64) -> Vec<f64> {
    match quartis(y) {
        Some((q1, q3)) => {
            let iqr = q3 - q1;
            let (lo, hi) = (q1 - mult * iqr, q3 + mult * iqr);
            y.iter().map(|v| v.clamp(lo, hi)).collect()
        }
        None => y.to_vec(),
    }
}

/// Erro quadrático médio (RMSE) entre duas séries de mesmo tamanho.
#[must_use]
pub fn rmse(real: &[f64], previsto: &[f64]) -> f64 {
    let n = real.len().min(previsto.len());
    if n == 0 {
        return 0.0;
    }
    let soma: f64 = (0..n).map(|i| (real[i] - previsto[i]).powi(2)).sum();
    (soma / n as f64).sqrt()
}

#[cfg(test)]
#[allow(clippy::float_cmp)] // comparações exatas de valores construídos deterministicamente
mod testes {
    use super::{
        limpar_outliers_iqr, media_movel, projetar, regressao_linear, rmse, suavizacao_exponencial,
    };

    #[test]
    fn regressao_de_reta_perfeita() {
        // y = 2x + 1 → slope 2, intercepto 1, correlação 1.
        let y: Vec<f64> = (0..10).map(|x| 2.0 * f64::from(x) + 1.0).collect();
        let t = regressao_linear(&y);
        assert!((t.slope - 2.0).abs() < 1e-9);
        assert!((t.intercepto - 1.0).abs() < 1e-9);
        assert!((t.correlacao - 1.0).abs() < 1e-9);
        assert!((projetar(&t, 10) - 21.0).abs() < 1e-9);
    }

    #[test]
    fn regressao_serie_curta_ou_constante() {
        assert_eq!(regressao_linear(&[]).slope, 0.0);
        assert_eq!(regressao_linear(&[5.0]).intercepto, 5.0);
        // Constante → slope 0, correlação 0 (sem variância).
        let t = regressao_linear(&[3.0, 3.0, 3.0, 3.0]);
        assert_eq!(t.slope, 0.0);
        assert_eq!(t.correlacao, 0.0);
    }

    #[test]
    fn media_movel_janela_7() {
        let y = [1.0, 2.0, 3.0, 4.0, 5.0];
        let m = media_movel(&y, 3);
        assert!((m[0] - 1.0).abs() < 1e-9); // só o primeiro
        assert!((m[2] - 2.0).abs() < 1e-9); // (1+2+3)/3
        assert!((m[4] - 4.0).abs() < 1e-9); // (3+4+5)/3
    }

    #[test]
    fn suavizacao_segue_a_serie() {
        let y = [10.0, 10.0, 10.0];
        let s = suavizacao_exponencial(&y, 0.5);
        assert!(s.iter().all(|&v| (v - 10.0).abs() < 1e-9));
    }

    #[test]
    fn iqr_limita_outlier() {
        let y = [10.0, 11.0, 9.0, 10.0, 12.0, 8.0, 1000.0, 10.0];
        let limpo = limpar_outliers_iqr(&y, 1.5);
        assert!(limpo.iter().all(|&v| v < 1000.0)); // o 1000 foi clampado
    }

    #[test]
    fn rmse_zero_quando_igual() {
        assert!((rmse(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0])).abs() < 1e-9);
    }
}
