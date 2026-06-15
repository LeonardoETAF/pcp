//! Precedência da classificação (doc 02 §2): F (precedência 1) → D (2) → N (3). Se nenhuma
//! se aplica, o produto é candidato à curva de Pareto (A/B/C).

use chrono::{Duration, NaiveDate};

use super::classificador::ProdutoParaClassificar;
use super::parametros::ParametrosClassificacao;
use crate::tipos::ClasseAbc;

/// Resultado da precedência: uma classe já definida (F/D/N) ou candidato a Pareto.
#[derive(Debug, Clone, Copy)]
pub(super) enum PreClasse {
    Definida(ClasseAbc),
    CandidatoPareto,
}

/// Aplica a precedência F → D → N a um produto.
pub(super) fn pre_classificar(
    produto: &ProdutoParaClassificar,
    data_ref: NaiveDate,
    params: &ParametrosClassificacao,
) -> PreClasse {
    // F — fora de linha (precedência 1, doc 02 §2.1).
    if produto.fora_de_linha {
        return PreClasse::Definida(ClasseAbc::F);
    }
    // D — sem nenhuma venda nos últimos `janela_classe_d_dias` (precedência 2, doc 02 §2.2).
    let limite_d = data_ref - Duration::days(params.janela_classe_d_dias);
    let sem_venda_recente = produto.ultima_venda.is_none_or(|venda| venda < limite_d);
    if sem_venda_recente {
        return PreClasse::Definida(ClasseAbc::D);
    }
    // N — primeira venda há menos de `janela_produto_novo_dias` (precedência 3, doc 02 §2.3).
    let limite_n = data_ref - Duration::days(params.janela_produto_novo_dias);
    let eh_novo = produto.primeira_venda.is_some_and(|venda| venda > limite_n);
    if eh_novo {
        return PreClasse::Definida(ClasseAbc::N);
    }
    PreClasse::CandidatoPareto
}

#[cfg(test)]
mod testes {
    use super::{pre_classificar, PreClasse};
    use crate::classificacao::classificador::ProdutoParaClassificar;
    use crate::classificacao::parametros::{FatoresAbc, ParametrosClassificacao};
    use crate::tipos::{ClasseAbc, CodigoEstoque};
    use chrono::NaiveDate;

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

    fn produto(
        fora: bool,
        primeira: Option<NaiveDate>,
        ultima: Option<NaiveDate>,
    ) -> ProdutoParaClassificar {
        ProdutoParaClassificar {
            codigo_estoque: CodigoEstoque::novo("X"),
            fora_de_linha: fora,
            primeira_venda: primeira,
            ultima_venda: ultima,
            volume_janela_abc: 1000,
        }
    }

    fn definida(pre: PreClasse) -> Option<ClasseAbc> {
        match pre {
            PreClasse::Definida(c) => Some(c),
            PreClasse::CandidatoPareto => None,
        }
    }

    #[test]
    fn f_tem_precedencia_mesmo_com_vendas() {
        // Fora de linha com venda recente e volume alto: ainda é F (precedência 1).
        let p = produto(true, d(2024, 1, 1), d(2026, 6, 10));
        assert_eq!(
            definida(pre_classificar(&p, data(), &params())),
            Some(ClasseAbc::F)
        );
    }

    #[test]
    fn d_quando_sem_venda_ha_mais_de_180_dias() {
        let p = produto(false, d(2024, 1, 1), d(2025, 11, 1)); // > 180 dias antes de 15/06/2026
        assert_eq!(
            definida(pre_classificar(&p, data(), &params())),
            Some(ClasseAbc::D)
        );
    }

    #[test]
    fn d_quando_nunca_vendeu() {
        let p = produto(false, None, None);
        assert_eq!(
            definida(pre_classificar(&p, data(), &params())),
            Some(ClasseAbc::D)
        );
    }

    #[test]
    fn n_quando_primeira_venda_ha_menos_de_60_dias() {
        // Primeira venda 26 dias antes; venda recente (não é D).
        let p = produto(false, d(2026, 5, 20), d(2026, 6, 10));
        assert_eq!(
            definida(pre_classificar(&p, data(), &params())),
            Some(ClasseAbc::N)
        );
    }

    #[test]
    fn candidato_pareto_quando_maduro_e_ativo() {
        // Ativo, primeira venda antiga, venda recente -> não é F/D/N.
        let p = produto(false, d(2024, 1, 1), d(2026, 6, 10));
        assert!(definida(pre_classificar(&p, data(), &params())).is_none());
    }

    #[test]
    fn d_vence_n_sem_venda_recente() {
        // Sem venda há mais de 180 dias: é D mesmo que a 1a venda fosse "nova" no passado.
        let p = produto(false, d(2025, 10, 1), d(2025, 10, 1));
        assert_eq!(
            definida(pre_classificar(&p, data(), &params())),
            Some(ClasseAbc::D)
        );
    }
}
