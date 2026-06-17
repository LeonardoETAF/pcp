//! Máquina de estados da sugestão de ciclo de vida (doc 04 §3.4):
//! `Gerada` → `EmAnalise` → `Aplicada` / `Recusada` / `Expirada`. Terminais não transitam.

use thiserror::Error;

/// Estado da sugestão de ciclo de vida.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoCicloVida {
    Gerada,
    EmAnalise,
    Aplicada,
    Recusada,
    Expirada,
}

/// Transição não permitida pela máquina de estados.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("transição inválida de {de:?} para {para:?}")]
pub struct ErroTransicao {
    pub de: EstadoCicloVida,
    pub para: EstadoCicloVida,
}

impl EstadoCicloVida {
    /// Código canônico (estável) para persistir/expor.
    #[must_use]
    pub const fn codigo(self) -> &'static str {
        match self {
            EstadoCicloVida::Gerada => "gerada",
            EstadoCicloVida::EmAnalise => "em_analise",
            EstadoCicloVida::Aplicada => "aplicada",
            EstadoCicloVida::Recusada => "recusada",
            EstadoCicloVida::Expirada => "expirada",
        }
    }

    /// Converte do código persistido; `None` se desconhecido.
    #[must_use]
    pub fn tentar_de(codigo: &str) -> Option<Self> {
        match codigo {
            "gerada" => Some(EstadoCicloVida::Gerada),
            "em_analise" => Some(EstadoCicloVida::EmAnalise),
            "aplicada" => Some(EstadoCicloVida::Aplicada),
            "recusada" => Some(EstadoCicloVida::Recusada),
            "expirada" => Some(EstadoCicloVida::Expirada),
            _ => None,
        }
    }

    /// `true` para estados terminais (aplicada/recusada/expirada).
    #[must_use]
    pub fn eh_terminal(self) -> bool {
        matches!(
            self,
            EstadoCicloVida::Aplicada | EstadoCicloVida::Recusada | EstadoCicloVida::Expirada
        )
    }

    /// `true` se a transição `self -> destino` é permitida.
    #[must_use]
    pub fn pode_ir_para(self, destino: EstadoCicloVida) -> bool {
        use EstadoCicloVida as E;
        matches!(
            (self, destino),
            (E::Gerada, E::EmAnalise | E::Expirada)
                | (E::EmAnalise, E::Aplicada | E::Recusada | E::Expirada)
        )
    }
}

/// Aplica uma transição validando a máquina de estados (doc 04 §3.4).
///
/// # Errors
/// [`ErroTransicao`] se a transição não for permitida.
pub fn transicionar(
    de: EstadoCicloVida,
    para: EstadoCicloVida,
) -> Result<EstadoCicloVida, ErroTransicao> {
    if de.pode_ir_para(para) {
        Ok(para)
    } else {
        Err(ErroTransicao { de, para })
    }
}

#[cfg(test)]
mod testes {
    use super::{transicionar, EstadoCicloVida};

    #[test]
    fn fluxo_feliz() {
        assert!(transicionar(EstadoCicloVida::Gerada, EstadoCicloVida::EmAnalise).is_ok());
        assert!(transicionar(EstadoCicloVida::EmAnalise, EstadoCicloVida::Aplicada).is_ok());
        assert!(transicionar(EstadoCicloVida::EmAnalise, EstadoCicloVida::Recusada).is_ok());
        assert!(transicionar(EstadoCicloVida::Gerada, EstadoCicloVida::Expirada).is_ok());
        assert!(transicionar(EstadoCicloVida::EmAnalise, EstadoCicloVida::Expirada).is_ok());
    }

    #[test]
    fn nao_pula_em_analise() {
        // Gerada não vai direto para Aplicada/Recusada (passa por EmAnalise).
        assert!(transicionar(EstadoCicloVida::Gerada, EstadoCicloVida::Aplicada).is_err());
        assert!(transicionar(EstadoCicloVida::Gerada, EstadoCicloVida::Recusada).is_err());
    }

    #[test]
    fn terminais_nao_transitam() {
        for terminal in [
            EstadoCicloVida::Aplicada,
            EstadoCicloVida::Recusada,
            EstadoCicloVida::Expirada,
        ] {
            assert!(terminal.eh_terminal());
            assert!(transicionar(terminal, EstadoCicloVida::EmAnalise).is_err());
        }
        assert!(!EstadoCicloVida::Gerada.eh_terminal());
        assert!(!EstadoCicloVida::EmAnalise.eh_terminal());
    }
}
