//! Orquestrador da classificação (doc 02 §2): aplica a precedência F→D→N e, para os demais,
//! a curva de Pareto (A/B/C). Cada produto recebe exatamente UMA classe (doc 08 §4).

use std::collections::HashMap;

use chrono::NaiveDate;

use super::parametros::ParametrosClassificacao;
use super::pareto;
use super::precedencia::{self, PreClasse};
use crate::tipos::{ClasseAbc, CodigoEstoque};

/// Dados agregados de um produto para classificação. Os agregados (volume na janela ABC,
/// primeira/última venda) vêm pré-calculados pelo chamador — agregação pesada fica no banco
/// (CLAUDE.md §15); aqui mora só a regra.
#[derive(Debug, Clone)]
pub struct ProdutoParaClassificar {
    pub codigo_estoque: CodigoEstoque,
    /// Flag do snapshot de estoque mais recente (doc 02 §2.1).
    pub fora_de_linha: bool,
    /// Primeira venda no histórico inteiro, sem janela (doc 02 §2.3). `None` = nunca vendeu.
    pub primeira_venda: Option<NaiveDate>,
    /// Venda mais recente (doc 02 §2.2). `None` = nunca vendeu.
    pub ultima_venda: Option<NaiveDate>,
    /// Volume somado na janela ABC de `janela_abc_dias` (doc 02 §2.4).
    pub volume_janela_abc: i64,
}

/// Resultado da classificação de um produto (doc 02 §2.5 / doc 04 §3.1).
#[derive(Debug, Clone, PartialEq)]
pub struct ResultadoClassificacao {
    pub codigo_estoque: CodigoEstoque,
    pub classe: ClasseAbc,
    pub fator_estoque: f64,
    /// Volume da janela ABC usado na classificação (nome honesto — doc 08 §2.3).
    pub volume_janela: i64,
    /// Posição na curva de Pareto, em %; `None` para F/D/N.
    pub percentual_acumulado: Option<f64>,
}

/// Classifica todos os produtos para a `data_ref`, na precedência F→D→N→Pareto (doc 02 §2).
/// Devolve um resultado por produto, na mesma ordem da entrada.
#[must_use]
pub fn classificar(
    produtos: &[ProdutoParaClassificar],
    data_ref: NaiveDate,
    params: &ParametrosClassificacao,
) -> Vec<ResultadoClassificacao> {
    let pre: Vec<PreClasse> = produtos
        .iter()
        .map(|produto| precedencia::pre_classificar(produto, data_ref, params))
        .collect();

    let candidatos: Vec<usize> = (0..produtos.len())
        .filter(|&i| matches!(pre[i], PreClasse::CandidatoPareto))
        .collect();

    let pareto = pareto::classificar(produtos, &candidatos, params);
    let mut por_indice: HashMap<usize, pareto::ParetoResultado> =
        candidatos.into_iter().zip(pareto).collect();

    produtos
        .iter()
        .enumerate()
        .map(|(i, produto)| {
            let (classe, percentual_acumulado) = match pre[i] {
                PreClasse::Definida(classe) => (classe, None),
                PreClasse::CandidatoPareto => {
                    let resultado = por_indice.remove(&i).unwrap_or(pareto::ParetoResultado {
                        classe: ClasseAbc::C,
                        percentual_acumulado: 100.0,
                    });
                    (resultado.classe, Some(resultado.percentual_acumulado))
                }
            };
            ResultadoClassificacao {
                codigo_estoque: produto.codigo_estoque.clone(),
                classe,
                fator_estoque: params.fatores.de(classe),
                volume_janela: produto.volume_janela_abc,
                percentual_acumulado,
            }
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::{classificar, ProdutoParaClassificar, ResultadoClassificacao};
    use crate::classificacao::parametros::{FatoresAbc, ParametrosClassificacao};
    use crate::tipos::{ClasseAbc, CodigoEstoque};
    use chrono::NaiveDate;
    use std::collections::HashSet;

    fn data() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 6, 15).expect("data válida")
    }

    fn d(ano: i32, mes: u32, dia: u32) -> Option<NaiveDate> {
        NaiveDate::from_ymd_opt(ano, mes, dia)
    }

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

    fn prod(
        codigo: &str,
        fora: bool,
        primeira: Option<NaiveDate>,
        ultima: Option<NaiveDate>,
        volume: i64,
    ) -> ProdutoParaClassificar {
        ProdutoParaClassificar {
            codigo_estoque: CodigoEstoque::novo(codigo),
            fora_de_linha: fora,
            primeira_venda: primeira,
            ultima_venda: ultima,
            volume_janela_abc: volume,
        }
    }

    fn classe_de(res: &[ResultadoClassificacao], codigo: &str) -> ClasseAbc {
        res.iter()
            .find(|r| r.codigo_estoque.como_str() == codigo)
            .expect("produto presente no resultado")
            .classe
    }

    /// Cenário cobrindo as 6 classes + a precedência (doc 02 §2, doc 08 §4).
    fn cenario() -> Vec<ProdutoParaClassificar> {
        vec![
            // F — fora de linha, mesmo com volume altíssimo (precedência sobre Pareto).
            prod("F1", true, d(2024, 1, 1), d(2026, 6, 10), 9999),
            // D — sem venda há mais de 180 dias (mesmo com volume na janela).
            prod("D1", false, d(2024, 1, 1), d(2025, 11, 1), 500),
            // N — primeira venda há menos de 60 dias.
            prod("N1", false, d(2026, 5, 20), d(2026, 6, 10), 50),
            // Pareto: total 100 -> 70%(A), 90%(B), 96%(C), 100%(C).
            prod("PA", false, d(2024, 1, 1), d(2026, 6, 10), 70),
            prod("PB", false, d(2024, 1, 1), d(2026, 6, 10), 20),
            prod("PC1", false, d(2024, 1, 1), d(2026, 6, 10), 6),
            prod("PC2", false, d(2024, 1, 1), d(2026, 6, 10), 4),
        ]
    }

    #[test]
    fn classifica_as_seis_classes_com_precedencia() {
        let res = classificar(&cenario(), data(), &params());
        assert_eq!(classe_de(&res, "F1"), ClasseAbc::F);
        assert_eq!(classe_de(&res, "D1"), ClasseAbc::D);
        assert_eq!(classe_de(&res, "N1"), ClasseAbc::N);
        assert_eq!(classe_de(&res, "PA"), ClasseAbc::A);
        assert_eq!(classe_de(&res, "PB"), ClasseAbc::B);
        assert_eq!(classe_de(&res, "PC1"), ClasseAbc::C);
        assert_eq!(classe_de(&res, "PC2"), ClasseAbc::C);
    }

    #[test]
    fn fator_de_estoque_acompanha_a_classe() {
        let res = classificar(&cenario(), data(), &params());
        let fator = |cod| {
            res.iter()
                .find(|r| r.codigo_estoque.como_str() == cod)
                .unwrap()
                .fator_estoque
        };
        assert!((fator("PA") - 1.20).abs() < f64::EPSILON); // A
        assert!((fator("F1") - 0.10).abs() < f64::EPSILON); // F
        assert!((fator("D1") - 0.30).abs() < f64::EPSILON); // D
        assert!((fator("N1") - 0.80).abs() < f64::EPSILON); // N
    }

    #[test]
    fn invariante_uma_classe_por_produto() {
        let entrada = cenario();
        let res = classificar(&entrada, data(), &params());
        // Um resultado por produto, sem duplicar nem perder códigos (doc 08 §4).
        assert_eq!(res.len(), entrada.len());
        let codigos: HashSet<&str> = res.iter().map(|r| r.codigo_estoque.como_str()).collect();
        assert_eq!(codigos.len(), entrada.len());
    }

    #[test]
    fn invariante_pareto_soma_100() {
        let res = classificar(&cenario(), data(), &params());
        // Soma dos percentuais de Pareto = 100 -> o maior acumulado dos A/B/C é 100 (doc 08 §4).
        let maximo = res
            .iter()
            .filter_map(|r| r.percentual_acumulado)
            .fold(0.0_f64, f64::max);
        assert!((maximo - 100.0).abs() < 1e-9, "máximo acumulado = {maximo}");
        // F/D/N não entram na curva (percentual_acumulado = None).
        for cod in ["F1", "D1", "N1"] {
            let r = res
                .iter()
                .find(|r| r.codigo_estoque.como_str() == cod)
                .unwrap();
            assert!(r.percentual_acumulado.is_none());
        }
    }
}
