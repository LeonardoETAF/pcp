//! Sazonalidade dinâmica (doc 02 §4): fator multiplicador por mês, gatilho de recálculo e
//! lookup com fallback. Funções puras — a agregação das vendas e a persistência ficam no
//! `pcp-engine`/`pcp-db` (CLAUDE.md §2/§15).

use chrono::{Datelike, NaiveDate};

/// Limiares da sazonalidade (doc 02 §4). Originados de pcp-config (§2/§13).
#[derive(Debug, Clone, Copy)]
pub struct ParametrosSazonalidade {
    /// Piso do fator (doc 02 §4.1: 0.5).
    pub clamp_min: f64,
    /// Teto do fator (doc 02 §4.1: 2.0).
    pub clamp_max: f64,
    /// Dias sem atualizar que forçam recálculo (doc 02 §4.2: 30).
    pub atualizar_apos_dias: i64,
}

/// Calcula o fator sazonal de um mês (doc 02 §4.1):
/// `fator = media_diaria_mes / media_diaria_ano`, com `CLAMP(0.5, 2.0)`.
/// Sem base anual (`media_ano <= 0`), retorna 1.0 (neutro).
#[must_use]
pub fn calcular_fator(media_mes: f64, media_ano: f64, clamp_min: f64, clamp_max: f64) -> f64 {
    if media_ano <= 0.0 {
        return 1.0;
    }
    (media_mes / media_ano).clamp(clamp_min, clamp_max)
}

/// Decide se os fatores devem ser recalculados (doc 02 §4.2): recalcula se nunca foram
/// calculados, se o mês mudou, ou se passaram mais de `atualizar_apos_dias` dias.
#[must_use]
pub fn deve_recalcular(
    ultima_atualizacao: Option<NaiveDate>,
    hoje: NaiveDate,
    atualizar_apos_dias: i64,
) -> bool {
    match ultima_atualizacao {
        None => true,
        Some(ultima) => {
            ultima.month() != hoje.month() || (hoje - ultima).num_days() > atualizar_apos_dias
        }
    }
}

/// Os 12 fatores sazonais (índice 0 = janeiro). Aplicados em §3 e §7 via [`Self::obter_fator`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FatoresSazonais {
    fatores: [f64; 12],
}

impl FatoresSazonais {
    /// Cria a partir dos 12 fatores (índice 0 = mês 1).
    #[must_use]
    pub fn novo(fatores: [f64; 12]) -> Self {
        Self { fatores }
    }

    /// Fator do mês (1–12), com fallback 1.0 para mês fora do intervalo (doc 04 §6.1).
    #[must_use]
    pub fn obter_fator(&self, mes: u32) -> f64 {
        if (1..=12).contains(&mes) {
            self.fatores[mes as usize - 1]
        } else {
            1.0
        }
    }
}

#[cfg(test)]
mod testes {
    use super::{calcular_fator, deve_recalcular, FatoresSazonais};
    use chrono::NaiveDate;

    fn data(ano: i32, mes: u32, dia: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(ano, mes, dia).expect("data válida")
    }

    #[test]
    fn fator_normal_sem_clamp() {
        assert!((calcular_fator(150.0, 100.0, 0.5, 2.0) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn fator_clampa_no_teto_e_no_piso() {
        assert!((calcular_fator(300.0, 100.0, 0.5, 2.0) - 2.0).abs() < 1e-9); // 3.0 -> 2.0
        assert!((calcular_fator(10.0, 100.0, 0.5, 2.0) - 0.5).abs() < 1e-9); // 0.1 -> 0.5
    }

    #[test]
    fn fator_neutro_sem_base_anual() {
        assert!((calcular_fator(50.0, 0.0, 0.5, 2.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn gatilho_recalcula_quando_nunca_calculado() {
        assert!(deve_recalcular(None, data(2026, 6, 15), 30));
    }

    #[test]
    fn gatilho_recalcula_quando_mes_mudou() {
        // Última em maio, hoje em junho -> mês mudou.
        assert!(deve_recalcular(
            Some(data(2026, 5, 20)),
            data(2026, 6, 15),
            30
        ));
    }

    #[test]
    fn gatilho_recalcula_quando_passou_de_30_dias() {
        // Mesmo mês (junho), porém um ano antes -> > 30 dias.
        assert!(deve_recalcular(
            Some(data(2025, 6, 20)),
            data(2026, 6, 15),
            30
        ));
    }

    #[test]
    fn gatilho_nao_recalcula_quando_recente_no_mesmo_mes() {
        assert!(!deve_recalcular(
            Some(data(2026, 6, 1)),
            data(2026, 6, 15),
            30
        ));
    }

    #[test]
    fn obter_fator_e_fallback() {
        let f = FatoresSazonais::novo([
            1.25, 0.99, 1.00, 0.87, 0.81, 0.62, 0.64, 0.63, 0.71, 0.90, 0.67, 2.00,
        ]);
        assert!((f.obter_fator(1) - 1.25).abs() < 1e-9); // janeiro
        assert!((f.obter_fator(12) - 2.00).abs() < 1e-9); // dezembro
        assert!((f.obter_fator(0) - 1.0).abs() < 1e-9); // fora do intervalo -> fallback
        assert!((f.obter_fator(13) - 1.0).abs() < 1e-9);
    }
}
