//! Alertas de produção (doc 02 §6): prioridade por % do recomendado, elevação de classe A
//! e ordenação da fila de produção. Função pura.
// Percentual do recomendado: quantidades pequenas, casts exatos.
#![allow(clippy::cast_precision_loss)]

use crate::recomendacao::qtd_sugerida;
use crate::tipos::{ClasseAbc, CodigoEstoque};

/// Prioridade do alerta (doc 02 §6.2). A ordem (Crítico < Alto < Médio) ordena a fila (§6.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Prioridade {
    Critico,
    Alto,
    Medio,
}

/// Limiares dos alertas (doc 02 §6.2/§6.3 / §11). Originados de pcp-config (§2/§13).
#[derive(Debug, Clone, Copy)]
pub struct ParametrosAlerta {
    pub critico_pct: f64,
    pub alto_pct: f64,
    pub medio_pct: f64,
    pub elevar_classe_a: bool,
}

/// Dados de um produto candidato a alerta (doc 02 §6.1).
#[derive(Debug, Clone)]
pub struct EntradaAlerta {
    pub codigo_estoque: CodigoEstoque,
    pub classe: ClasseAbc,
    pub fora_de_linha: bool,
    pub media_diaria: f64,
    pub qtd_disponivel: i64,
    pub estoque_total_recomendado: i64,
    pub cobertura_dias: f64,
}

/// Um alerta de produção (doc 02 §6.4). A prioridade tem campo próprio (não o `configuracao`).
#[derive(Debug, Clone, PartialEq)]
pub struct Alerta {
    pub codigo_estoque: CodigoEstoque,
    pub prioridade: Prioridade,
    pub classe: ClasseAbc,
    pub qtd_sugerida: i64,
    pub cobertura_dias: f64,
}

/// Gera os alertas do dia, já ordenados para a fila de produção (doc 02 §6.5):
/// prioridade → classe (A primeiro) → `qtd_sugerida` decrescente.
#[must_use]
pub fn gerar_alertas(entradas: &[EntradaAlerta], params: &ParametrosAlerta) -> Vec<Alerta> {
    let mut alertas: Vec<Alerta> = entradas.iter().filter_map(|e| avaliar(e, params)).collect();
    alertas.sort_by(|a, b| {
        a.prioridade
            .cmp(&b.prioridade)
            .then(a.classe.cmp(&b.classe))
            .then(b.qtd_sugerida.cmp(&a.qtd_sugerida))
    });
    alertas
}

fn avaliar(entrada: &EntradaAlerta, params: &ParametrosAlerta) -> Option<Alerta> {
    // Universo (doc 02 §6.1): fora de linha e sem histórico não geram alerta.
    if entrada.fora_de_linha || entrada.media_diaria <= 0.0 {
        return None;
    }
    let base = prioridade_base(
        entrada.qtd_disponivel,
        entrada.estoque_total_recomendado,
        params,
    )?;
    // Elevação de classe A (doc 02 §6.3).
    let prioridade = if params.elevar_classe_a && entrada.classe == ClasseAbc::A {
        elevar(base)
    } else {
        base
    };
    Some(Alerta {
        codigo_estoque: entrada.codigo_estoque.clone(),
        prioridade,
        classe: entrada.classe,
        qtd_sugerida: qtd_sugerida(
            entrada.estoque_total_recomendado,
            entrada.qtd_disponivel,
            entrada.fora_de_linha,
            entrada.media_diaria,
        ),
        cobertura_dias: entrada.cobertura_dias,
    })
}

/// Prioridade base por % do recomendado (doc 02 §6.2). `None` acima de `medio_pct` (sem alerta).
fn prioridade_base(
    disponivel: i64,
    recomendado: i64,
    params: &ParametrosAlerta,
) -> Option<Prioridade> {
    let disp = disponivel as f64;
    let rec = recomendado as f64;
    if disponivel <= 0 || disp < params.critico_pct * rec {
        Some(Prioridade::Critico)
    } else if disp < params.alto_pct * rec {
        Some(Prioridade::Alto)
    } else if disp < params.medio_pct * rec {
        Some(Prioridade::Medio)
    } else {
        None
    }
}

/// Elevação de classe A (doc 02 §6.3): ALTO→CRÍTICO, MÉDIO→ALTO.
fn elevar(prioridade: Prioridade) -> Prioridade {
    match prioridade {
        Prioridade::Alto | Prioridade::Critico => Prioridade::Critico,
        Prioridade::Medio => Prioridade::Alto,
    }
}

#[cfg(test)]
mod testes {
    use super::{gerar_alertas, EntradaAlerta, ParametrosAlerta, Prioridade};
    use crate::tipos::{ClasseAbc, CodigoEstoque};

    fn params() -> ParametrosAlerta {
        ParametrosAlerta {
            critico_pct: 0.20,
            alto_pct: 0.50,
            medio_pct: 0.80,
            elevar_classe_a: true,
        }
    }

    fn entrada(
        codigo: &str,
        classe: ClasseAbc,
        disponivel: i64,
        fora: bool,
        media: f64,
    ) -> EntradaAlerta {
        EntradaAlerta {
            codigo_estoque: CodigoEstoque::novo(codigo),
            classe,
            fora_de_linha: fora,
            media_diaria: media,
            qtd_disponivel: disponivel,
            estoque_total_recomendado: 100,
            cobertura_dias: 10.0,
        }
    }

    fn so(entrada: EntradaAlerta) -> Vec<super::Alerta> {
        gerar_alertas(&[entrada], &params())
    }

    #[test]
    fn fora_de_linha_nunca_gera_alerta() {
        // Mesmo zerado (que seria crítico), fora de linha não entra no universo (§6.1).
        let r = so(entrada("F", ClasseAbc::C, 0, true, 10.0));
        assert!(r.is_empty());
    }

    #[test]
    fn sem_historico_nao_gera_alerta() {
        let r = so(entrada("S", ClasseAbc::C, 0, false, 0.0));
        assert!(r.is_empty());
    }

    #[test]
    fn acima_de_80_pct_nao_gera_alerta() {
        let r = so(entrada("OK", ClasseAbc::C, 90, false, 10.0)); // 90% do recomendado
        assert!(r.is_empty());
    }

    #[test]
    fn faixas_de_prioridade() {
        assert_eq!(
            so(entrada("c", ClasseAbc::C, 10, false, 10.0))[0].prioridade,
            Prioridade::Critico
        );
        assert_eq!(
            so(entrada("c", ClasseAbc::C, 40, false, 10.0))[0].prioridade,
            Prioridade::Alto
        );
        assert_eq!(
            so(entrada("c", ClasseAbc::C, 70, false, 10.0))[0].prioridade,
            Prioridade::Medio
        );
    }

    #[test]
    fn classe_a_eleva_a_prioridade() {
        // Classe A com 40% (base Alto) -> elevado a Crítico (§6.3).
        assert_eq!(
            so(entrada("a", ClasseAbc::A, 40, false, 10.0))[0].prioridade,
            Prioridade::Critico
        );
        // Classe A com 70% (base Médio) -> elevado a Alto.
        assert_eq!(
            so(entrada("a", ClasseAbc::A, 70, false, 10.0))[0].prioridade,
            Prioridade::Alto
        );
    }

    #[test]
    fn qtd_sugerida_nunca_negativa() {
        let r = so(entrada("c", ClasseAbc::C, 10, false, 10.0));
        assert!(r[0].qtd_sugerida >= 0);
        assert_eq!(r[0].qtd_sugerida, 90); // 100 - 10
    }

    #[test]
    fn ordena_fila_por_prioridade_classe_e_qtd() {
        let entradas = [
            entrada("p3", ClasseAbc::C, 70, false, 10.0), // Médio
            entrada("p1", ClasseAbc::C, 10, false, 10.0), // Crítico, qtd 90
            entrada("p2", ClasseAbc::C, 40, false, 10.0), // Alto
            entrada("pA", ClasseAbc::A, 40, false, 10.0), // base Alto -> Crítico (classe A), qtd 60
        ];
        let r = gerar_alertas(&entradas, &params());
        let ordem: Vec<&str> = r.iter().map(|a| a.codigo_estoque.como_str()).collect();
        // Crítico primeiro; dentro de Crítico, classe A antes de C; depois Alto, depois Médio.
        assert_eq!(ordem, ["pA", "p1", "p2", "p3"]);
    }
}
