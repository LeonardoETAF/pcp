//! Máquina de estados da Solicitação de Produção (doc 03 §4.3):
//! `Pendente` → `Aprovada` → `EmProducao` → `Concluida`, com `Pendente` → `Recusada`.
//! Estados terminais (`Concluida`, `Recusada`) não transitam. Pura e testável (CLAUDE.md §11).

use thiserror::Error;

/// Estado de uma solicitação de produção.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstadoSolicitacao {
    Pendente,
    Aprovada,
    EmProducao,
    Concluida,
    Recusada,
}

/// Transição não permitida pela máquina de estados.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("transição inválida de {de:?} para {para:?}")]
pub struct ErroTransicao {
    pub de: EstadoSolicitacao,
    pub para: EstadoSolicitacao,
}

impl EstadoSolicitacao {
    /// Código canônico (estável) para persistir/expor.
    #[must_use]
    pub const fn codigo(self) -> &'static str {
        match self {
            EstadoSolicitacao::Pendente => "pendente",
            EstadoSolicitacao::Aprovada => "aprovada",
            EstadoSolicitacao::EmProducao => "em_producao",
            EstadoSolicitacao::Concluida => "concluida",
            EstadoSolicitacao::Recusada => "recusada",
        }
    }

    /// Converte do código persistido; `None` se desconhecido.
    #[must_use]
    pub fn tentar_de(codigo: &str) -> Option<Self> {
        match codigo {
            "pendente" => Some(EstadoSolicitacao::Pendente),
            "aprovada" => Some(EstadoSolicitacao::Aprovada),
            "em_producao" => Some(EstadoSolicitacao::EmProducao),
            "concluida" => Some(EstadoSolicitacao::Concluida),
            "recusada" => Some(EstadoSolicitacao::Recusada),
            _ => None,
        }
    }

    /// `true` para estados terminais (concluída/recusada).
    #[must_use]
    pub fn eh_terminal(self) -> bool {
        matches!(
            self,
            EstadoSolicitacao::Concluida | EstadoSolicitacao::Recusada
        )
    }

    /// `true` se a transição `self -> destino` é permitida (doc 03 §4.3).
    #[must_use]
    pub fn pode_ir_para(self, destino: EstadoSolicitacao) -> bool {
        use EstadoSolicitacao as E;
        matches!(
            (self, destino),
            (E::Pendente, E::Aprovada | E::Recusada)
                | (E::Aprovada, E::EmProducao)
                | (E::EmProducao, E::Concluida)
        )
    }
}

/// Aplica uma transição validando a máquina de estados (doc 03 §4.3).
///
/// # Errors
/// [`ErroTransicao`] se a transição não for permitida.
pub fn transicionar(
    de: EstadoSolicitacao,
    para: EstadoSolicitacao,
) -> Result<EstadoSolicitacao, ErroTransicao> {
    if de.pode_ir_para(para) {
        Ok(para)
    } else {
        Err(ErroTransicao { de, para })
    }
}

#[cfg(test)]
mod testes {
    use super::{transicionar, EstadoSolicitacao as E};

    #[test]
    fn fluxo_feliz() {
        assert!(transicionar(E::Pendente, E::Aprovada).is_ok());
        assert!(transicionar(E::Aprovada, E::EmProducao).is_ok());
        assert!(transicionar(E::EmProducao, E::Concluida).is_ok());
        assert!(transicionar(E::Pendente, E::Recusada).is_ok());
    }

    #[test]
    fn nao_pula_etapas() {
        assert!(transicionar(E::Pendente, E::EmProducao).is_err());
        assert!(transicionar(E::Pendente, E::Concluida).is_err());
        assert!(transicionar(E::Aprovada, E::Concluida).is_err());
        // Recusar só a partir de pendente.
        assert!(transicionar(E::Aprovada, E::Recusada).is_err());
    }

    #[test]
    fn terminais_nao_transitam() {
        for terminal in [E::Concluida, E::Recusada] {
            assert!(terminal.eh_terminal());
            assert!(transicionar(terminal, E::EmProducao).is_err());
            assert!(transicionar(terminal, E::Aprovada).is_err());
        }
        assert!(!E::Pendente.eh_terminal());
        assert!(!E::Aprovada.eh_terminal());
        assert!(!E::EmProducao.eh_terminal());
    }

    #[test]
    fn codigo_ida_e_volta() {
        for e in [
            E::Pendente,
            E::Aprovada,
            E::EmProducao,
            E::Concluida,
            E::Recusada,
        ] {
            assert_eq!(E::tentar_de(e.codigo()), Some(e));
        }
        assert_eq!(E::tentar_de("xpto"), None);
    }
}
