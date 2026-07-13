//! Sazonalidade dinâmica (doc 02 §4): fator multiplicador por mês, gatilho de recálculo e
//! lookup com fallback. Funções puras — a agregação das vendas e a persistência ficam no
//! `pcp-engine`/`pcp-db` (CLAUDE.md §2/§15).

// Médias diárias: totais/dias pequenos cabem exatos em f64.
#![allow(clippy::cast_precision_loss)]

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
    /// Mínimo de meses COM VENDA para o produto ter curva sazonal PRÓPRIA. Abaixo disso ele usa
    /// a curva global (decisão do dono, 2026-07-13 — ver `calcular_fatores_produto`).
    pub min_meses_com_venda_produto: usize,
}

/// Vendas de um mês de um produto (entrada do fator sazonal por produto).
#[derive(Debug, Clone, Copy)]
pub struct VendasMesProduto {
    pub mes: u32,
    pub total: f64,
    /// Dias COM venda no mês (dia parado não entra na média — doc 02 §3.1).
    pub dias: i64,
}

/// Os 12 fatores sazonais de UM produto (doc 02 §4, estendido por produto — decisão do dono,
/// 2026-07-13). Mesma fórmula do global (`média do mês ÷ média do ano`, com clamp), mas sobre a
/// série do próprio item.
///
/// Devolve `None` quando o histórico do produto NÃO sustenta uma curva própria — e aí o chamador
/// usa o fator GLOBAL. Isso não é preciosismo: com poucos meses de venda, um único pedido grande
/// num mês vira um fator absurdo, e o produto passaria o ano seguinte com estoque errado. É
/// preferível a curva da empresa a uma curva inventada.
///
/// Mês sem venda fica NEUTRO (1.0), não 0.5: ausência de dado não é prova de demanda baixa.
#[must_use]
pub fn calcular_fatores_produto(
    vendas: &[VendasMesProduto],
    min_meses_com_venda: usize,
    clamp_min: f64,
    clamp_max: f64,
) -> Option<[f64; 12]> {
    let meses_com_venda = vendas
        .iter()
        .filter(|v| v.dias > 0 && v.total > 0.0)
        .count();
    if meses_com_venda < min_meses_com_venda {
        return None;
    }
    let total_ano: f64 = vendas.iter().map(|v| v.total).sum();
    let dias_ano: i64 = vendas.iter().map(|v| v.dias).sum();
    if dias_ano <= 0 || total_ano <= 0.0 {
        return None;
    }
    let media_ano = total_ano / dias_ano as f64;

    let mut fatores = [1.0_f64; 12];
    for v in vendas {
        if (1..=12).contains(&v.mes) && v.dias > 0 {
            let media_mes = v.total / v.dias as f64;
            let indice = usize::try_from(v.mes - 1).unwrap_or(0);
            fatores[indice] = calcular_fator(media_mes, media_ano, clamp_min, clamp_max);
        }
    }
    Some(fatores)
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

#[cfg(test)]
mod testes_produto {
    use super::{calcular_fatores_produto, VendasMesProduto};

    fn vm(mes: u32, total: f64, dias: i64) -> VendasMesProduto {
        VendasMesProduto { mes, total, dias }
    }

    #[test]
    fn produto_com_historico_ganha_curva_propria() {
        // 12 meses vendendo 10/dia, exceto dezembro com 20/dia: dezembro = 2x a média.
        let mut v: Vec<_> = (1..=11).map(|m| vm(m, 200.0, 20)).collect();
        v.push(vm(12, 400.0, 20)); // 20/dia
        let f = calcular_fatores_produto(&v, 6, 0.5, 2.0).expect("deve ter curva própria");
        // media_ano = (11*200 + 400) / (12*20) = 2600/240 = 10.83
        assert!(f[11] > 1.5, "dezembro deve puxar para cima: {}", f[11]);
        assert!(f[0] < 1.0, "janeiro fica abaixo da média: {}", f[0]);
    }

    #[test]
    fn produto_esparso_cai_no_fator_global() {
        // Vendeu em 3 meses só — não sustenta curva própria (mínimo 6).
        let v = vec![vm(3, 100.0, 5), vm(7, 50.0, 2), vm(12, 900.0, 1)];
        assert!(calcular_fatores_produto(&v, 6, 0.5, 2.0).is_none());
    }

    #[test]
    fn clamp_protege_de_pico_absurdo() {
        // 11 meses fracos + dezembro com um pedido gigante num único dia: o clamp segura em 2.0.
        let mut v: Vec<_> = (1..=11).map(|m| vm(m, 100.0, 20)).collect();
        v.push(vm(12, 50_000.0, 1));
        let f = calcular_fatores_produto(&v, 6, 0.5, 2.0).expect("tem 12 meses com venda");
        assert!(
            (f[11] - 2.0).abs() < 1e-9,
            "dezembro deve bater no teto: {}",
            f[11]
        );
    }

    #[test]
    fn mes_sem_venda_fica_neutro_nao_zerado() {
        // Produto que não vende em janeiro: fator 1.0 (neutro), não 0.5 — ausência de dado não é
        // prova de demanda baixa.
        let v: Vec<_> = (2..=12).map(|m| vm(m, 200.0, 20)).collect();
        let f = calcular_fatores_produto(&v, 6, 0.5, 2.0).expect("11 meses com venda");
        assert!((f[0] - 1.0).abs() < 1e-9, "janeiro neutro: {}", f[0]);
    }
}
