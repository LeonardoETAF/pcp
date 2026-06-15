//! Status canônico hierárquico de estoque (doc 02 §5.2). Função pura.

use crate::tipos::ClasseAbc;

/// Status de estoque (doc 02 §5.2), na ordem de avaliação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusEstoque {
    SemEstoque,
    ForaDeLinha,
    SemHistorico,
    Critico,
    EstoqueBaixo,
    Baixo,
    Adequado,
    Alto,
    Excessivo,
}

/// Limiar de criticidade (cobertura em dias) por classe (doc 02 §5.2 / §11).
#[derive(Debug, Clone, Copy)]
pub struct LimiarCriticoDias {
    pub a: i32,
    pub b: i32,
    pub c: i32,
}

/// Dados de um produto para avaliação de status (doc 02 §5.2).
#[derive(Debug, Clone)]
pub struct EntradaStatus {
    pub classe: ClasseAbc,
    pub fora_de_linha: bool,
    pub media_diaria: f64,
    pub cobertura_dias: f64,
    pub qtd_disponivel: i64,
    pub estoque_minimo: i64,
    pub estoque_seguranca: i64,
    pub estoque_total_recomendado: i64,
}

/// Avalia o status hierárquico (doc 02 §5.2): a primeira condição verdadeira vence.
#[must_use]
pub fn status_estoque(entrada: &EntradaStatus, limiar: &LimiarCriticoDias) -> StatusEstoque {
    if entrada.qtd_disponivel <= 0 {
        return StatusEstoque::SemEstoque;
    }
    if entrada.fora_de_linha {
        return StatusEstoque::ForaDeLinha;
    }
    if entrada.media_diaria <= 0.0 {
        return StatusEstoque::SemHistorico;
    }
    // Criticidade por classe; classes sem limiar próprio usam o de C (default defensivo).
    let limite_critico = match entrada.classe {
        ClasseAbc::A => limiar.a,
        ClasseAbc::B => limiar.b,
        _ => limiar.c,
    };
    if entrada.cobertura_dias <= f64::from(limite_critico) {
        return StatusEstoque::Critico;
    }
    if entrada.qtd_disponivel < entrada.estoque_minimo {
        return StatusEstoque::EstoqueBaixo;
    }
    if entrada.qtd_disponivel < entrada.estoque_seguranca {
        return StatusEstoque::Baixo;
    }
    if entrada.qtd_disponivel <= entrada.estoque_total_recomendado {
        return StatusEstoque::Adequado;
    }
    // ALTO: disponivel <= recomendado × 1.5 (em inteiros: disponivel×2 <= recomendado×3).
    if entrada.qtd_disponivel.saturating_mul(2)
        <= entrada.estoque_total_recomendado.saturating_mul(3)
    {
        return StatusEstoque::Alto;
    }
    StatusEstoque::Excessivo
}

#[cfg(test)]
mod testes {
    use super::{status_estoque, EntradaStatus, LimiarCriticoDias, StatusEstoque};
    use crate::tipos::ClasseAbc;

    fn limiar() -> LimiarCriticoDias {
        LimiarCriticoDias { a: 15, b: 10, c: 5 }
    }

    fn base() -> EntradaStatus {
        EntradaStatus {
            classe: ClasseAbc::C,
            fora_de_linha: false,
            media_diaria: 10.0,
            cobertura_dias: 100.0,
            qtd_disponivel: 100,
            estoque_minimo: 50,
            estoque_seguranca: 30,
            estoque_total_recomendado: 80,
        }
    }

    #[test]
    fn sem_estoque_vence_tudo() {
        let e = EntradaStatus {
            qtd_disponivel: 0,
            fora_de_linha: true,
            ..base()
        };
        assert_eq!(status_estoque(&e, &limiar()), StatusEstoque::SemEstoque);
    }

    #[test]
    fn fora_de_linha_antes_de_sem_historico() {
        let e = EntradaStatus {
            fora_de_linha: true,
            media_diaria: 0.0,
            ..base()
        };
        assert_eq!(status_estoque(&e, &limiar()), StatusEstoque::ForaDeLinha);
    }

    #[test]
    fn sem_historico_quando_media_zero() {
        let e = EntradaStatus {
            media_diaria: 0.0,
            ..base()
        };
        assert_eq!(status_estoque(&e, &limiar()), StatusEstoque::SemHistorico);
    }

    #[test]
    fn critico_por_classe() {
        // Classe A: crítico se cobertura <= 15.
        let a = EntradaStatus {
            classe: ClasseAbc::A,
            cobertura_dias: 15.0,
            ..base()
        };
        assert_eq!(status_estoque(&a, &limiar()), StatusEstoque::Critico);
        // Classe C: 15 dias NÃO é crítico (limiar 5).
        let c = EntradaStatus {
            cobertura_dias: 15.0,
            ..base()
        };
        assert_ne!(status_estoque(&c, &limiar()), StatusEstoque::Critico);
    }

    #[test]
    fn faixas_de_quantidade() {
        let adequado = EntradaStatus {
            qtd_disponivel: 80,
            ..base()
        }; // == recomendado
        assert_eq!(
            status_estoque(&adequado, &limiar()),
            StatusEstoque::Adequado
        );

        let alto = EntradaStatus {
            qtd_disponivel: 120,
            ..base()
        }; // <= 80*1.5=120
        assert_eq!(status_estoque(&alto, &limiar()), StatusEstoque::Alto);

        let excessivo = EntradaStatus {
            qtd_disponivel: 121,
            ..base()
        };
        assert_eq!(
            status_estoque(&excessivo, &limiar()),
            StatusEstoque::Excessivo
        );
    }
}
