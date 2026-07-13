//! Consolidação de vendas por `(dt_ref, codigo_estoque)`, somando as variações LISO e
//! PERSONALIZADO do mesmo produto (doc 02 §1). Função pura, sem I/O.

use std::collections::HashMap;

use chrono::NaiveDate;

use crate::tipos::CodigoEstoque;

/// Uma linha de venda bruta: uma variação (LISO ou PERSONALIZADO) de um produto num dia.
#[derive(Debug, Clone)]
pub struct VendaBruta {
    pub dt_ref: NaiveDate,
    pub codigo_estoque: CodigoEstoque,
    pub qtd_vendida: i64,
    pub is_personalizado: bool,
}

/// Venda consolidada de um produto num dia (doc 02 §1):
/// `qtd_total = SUM(qtd_vendida)` e `houve_personalizado = BOOL_OR(is_personalizado)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendaConsolidada {
    pub dt_ref: NaiveDate,
    pub codigo_estoque: CodigoEstoque,
    pub qtd_total: i64,
    pub houve_personalizado: bool,
}

/// Consolida as vendas agrupando por `(dt_ref, codigo_estoque)` (doc 02 §1). A saída é
/// ordenada por `(dt_ref, codigo_estoque)` para resultado determinístico.
#[must_use]
pub fn consolidar(vendas: &[VendaBruta]) -> Vec<VendaConsolidada> {
    let mut mapa: HashMap<(NaiveDate, &CodigoEstoque), (i64, bool)> = HashMap::new();
    for venda in vendas {
        let entrada = mapa
            .entry((venda.dt_ref, &venda.codigo_estoque))
            .or_insert((0, false));
        entrada.0 += venda.qtd_vendida;
        entrada.1 |= venda.is_personalizado;
    }

    let mut consolidadas: Vec<VendaConsolidada> = mapa
        .into_iter()
        .map(
            |((dt_ref, codigo), (qtd_total, houve_personalizado))| VendaConsolidada {
                dt_ref,
                codigo_estoque: codigo.clone(),
                qtd_total,
                houve_personalizado,
            },
        )
        .collect();
    consolidadas.sort_by(|a, b| {
        a.dt_ref
            .cmp(&b.dt_ref)
            .then_with(|| a.codigo_estoque.cmp(&b.codigo_estoque))
    });
    consolidadas
}

#[cfg(test)]
mod testes {
    use super::{consolidar, VendaBruta};
    use crate::tipos::CodigoEstoque;
    use chrono::NaiveDate;

    fn dia() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 6, 15).expect("data válida")
    }

    fn venda(codigo: &str, qtd: i64, personalizado: bool) -> VendaBruta {
        VendaBruta {
            dt_ref: dia(),
            codigo_estoque: CodigoEstoque::novo(codigo),
            qtd_vendida: qtd,
            is_personalizado: personalizado,
        }
    }

    #[test]
    fn soma_variacoes_e_aplica_bool_or() {
        // Mesmo produto, mesmo dia: LISO (10) + PERSONALIZADO (3) -> 13, houve_personalizado.
        let entrada = [
            venda("6797", 10, false),
            venda("6797", 3, true),
            venda("10001", 7, false),
        ];
        let saida = consolidar(&entrada);
        assert_eq!(saida.len(), 2);

        let p6797 = &saida[saida
            .iter()
            .position(|v| v.codigo_estoque.como_str() == "6797")
            .unwrap()];
        assert_eq!(p6797.qtd_total, 13);
        assert!(p6797.houve_personalizado);

        let p10001 = &saida[saida
            .iter()
            .position(|v| v.codigo_estoque.como_str() == "10001")
            .unwrap()];
        assert_eq!(p10001.qtd_total, 7);
        assert!(!p10001.houve_personalizado);
    }

    #[test]
    fn separa_por_dia() {
        let outro_dia = NaiveDate::from_ymd_opt(2026, 6, 14).expect("data válida");
        let entrada = [
            venda("6797", 5, false),
            VendaBruta {
                dt_ref: outro_dia,
                codigo_estoque: CodigoEstoque::novo("6797"),
                qtd_vendida: 8,
                is_personalizado: false,
            },
        ];
        let saida = consolidar(&entrada);
        // Mesmo código, dias diferentes -> duas linhas, ordenadas por data.
        assert_eq!(saida.len(), 2);
        assert_eq!(saida[0].dt_ref, outro_dia);
        assert_eq!(saida[0].qtd_total, 8);
        assert_eq!(saida[1].dt_ref, dia());
        assert_eq!(saida[1].qtd_total, 5);
    }

    #[test]
    fn entrada_vazia_gera_saida_vazia() {
        assert!(consolidar(&[]).is_empty());
    }
}
