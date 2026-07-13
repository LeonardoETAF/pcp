//! Curva de Pareto para os candidatos A/B/C (doc 02 §2.4). Ordena por volume na janela ABC
//! (decrescente) e atribui a classe pelo percentual acumulado: A ≤ `pareto_a`,
//! B ≤ `pareto_b`, C acima.

use std::collections::HashMap;

use super::classificador::ProdutoParaClassificar;
use super::parametros::ParametrosClassificacao;
use crate::tipos::ClasseAbc;

/// Classe e posição (percentual acumulado) de um candidato na curva.
#[derive(Debug, Clone, Copy)]
pub(super) struct ParetoResultado {
    pub classe: ClasseAbc,
    pub percentual_acumulado: f64,
}

/// Classifica os candidatos (índices em `produtos`) pela curva de Pareto. Retorna um
/// resultado por candidato, na MESMA ordem de `candidatos`.
pub(super) fn classificar(
    produtos: &[ProdutoParaClassificar],
    candidatos: &[usize],
    params: &ParametrosClassificacao,
) -> Vec<ParetoResultado> {
    let total: i64 = candidatos
        .iter()
        .map(|&i| produtos[i].volume_janela_abc)
        .sum();

    // Ordena por volume decrescente; desempate por código para resultado determinístico.
    let mut ordem: Vec<usize> = candidatos.to_vec();
    ordem.sort_by(|&a, &b| {
        produtos[b]
            .volume_janela_abc
            .cmp(&produtos[a].volume_janela_abc)
            .then_with(|| produtos[a].codigo_estoque.cmp(&produtos[b].codigo_estoque))
    });

    let mut por_indice: HashMap<usize, ParetoResultado> = HashMap::with_capacity(ordem.len());
    let mut acumulado: i64 = 0;
    for &i in &ordem {
        acumulado += produtos[i].volume_janela_abc;
        let percentual_acumulado = if total > 0 {
            percentual(acumulado, total)
        } else {
            100.0
        };
        let classe = if percentual_acumulado <= params.pareto_a {
            ClasseAbc::A
        } else if percentual_acumulado <= params.pareto_b {
            ClasseAbc::B
        } else {
            ClasseAbc::C
        };
        por_indice.insert(
            i,
            ParetoResultado {
                classe,
                percentual_acumulado,
            },
        );
    }

    // Realinha à ordem original de `candidatos`.
    candidatos
        .iter()
        .map(|&i| {
            por_indice.remove(&i).unwrap_or(ParetoResultado {
                classe: ClasseAbc::C,
                percentual_acumulado: 100.0,
            })
        })
        .collect()
}

#[allow(clippy::cast_precision_loss)] // volumes (~milhões) cabem exatos em f64 (< 2^53)
fn percentual(parte: i64, total: i64) -> f64 {
    (parte as f64 / total as f64) * 100.0
}

#[cfg(test)]
mod testes {
    use super::{classificar, ProdutoParaClassificar};
    use crate::classificacao::parametros::{FatoresAbc, ParametrosClassificacao};
    use crate::tipos::{ClasseAbc, CodigoEstoque};
    use chrono::NaiveDate;

    fn params() -> ParametrosClassificacao {
        ParametrosClassificacao {
            janela_abc_dias: 540,
            janela_classe_d_dias: 180,
            janela_produto_novo_dias: 60,
            pareto_a: 80.0,
            pareto_b: 95.0,
            fatores: FatoresAbc {
                a: 1.20,
                b: 1.00,
                c: 0.80,
                d: 0.30,
                f: 0.10,
                n: 0.80,
            },
        }
    }

    fn produto(codigo: &str, volume: i64) -> ProdutoParaClassificar {
        ProdutoParaClassificar {
            codigo_estoque: CodigoEstoque::novo(codigo),
            fora_de_linha: false,
            primeira_venda: NaiveDate::from_ymd_opt(2024, 1, 1),
            ultima_venda: NaiveDate::from_ymd_opt(2026, 6, 10),
            volume_janela_abc: volume,
        }
    }

    #[test]
    fn corta_a_b_c_pelo_percentual_acumulado() {
        // Volumes (total 100): 70 -> 70% (A), +20 -> 90% (B), +6 -> 96% (C), +4 -> 100% (C).
        let produtos = [
            produto("PA", 70),
            produto("PB", 20),
            produto("PC1", 6),
            produto("PC2", 4),
        ];
        let candidatos = [0usize, 1, 2, 3];
        let r = classificar(&produtos, &candidatos, &params());

        assert_eq!(r[0].classe, ClasseAbc::A);
        assert_eq!(r[1].classe, ClasseAbc::B);
        assert_eq!(r[2].classe, ClasseAbc::C);
        assert_eq!(r[3].classe, ClasseAbc::C);

        // Invariante (doc 08 §4): o acumulado chega a 100%.
        let maximo = r
            .iter()
            .map(|x| x.percentual_acumulado)
            .fold(0.0_f64, f64::max);
        assert!((maximo - 100.0).abs() < 1e-9, "máximo acumulado = {maximo}");
    }

    #[test]
    fn total_zero_nao_quebra() {
        let produtos = [produto("Z", 0)];
        let r = classificar(&produtos, &[0], &params());
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].classe, ClasseAbc::C);
    }
}
