//! Conceito de auditoria de mudança de configuração: registra quem mudou, quando e o valor
//! anterior (CLAUDE.md §7.5 / doc 02 §11). A persistência real entra com a tela de
//! Configurações (prompt 3.3); aqui ficam só o tipo e a porta (trait).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Uma mudança aplicada a uma chave da configuração de negócio.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MudancaConfig {
    /// Caminho da constante alterada, ex.: `classificacao.pareto_a`.
    pub chave: String,
    /// Valor antes da mudança, serializado como texto.
    pub valor_anterior: String,
    /// Valor depois da mudança, serializado como texto.
    pub valor_novo: String,
    /// Identificador de quem aplicou a mudança.
    pub autor: String,
    /// Momento (UTC) em que a mudança foi aplicada.
    pub quando: DateTime<Utc>,
    /// Justificativa opcional informada pelo autor.
    pub justificativa: Option<String>,
}

/// Porta de auditoria de configuração: registra cada [`MudancaConfig`] numa trilha durável.
/// Será implementada por um adaptador em `pcp-db`; nunca aqui (este crate não faz I/O).
pub trait AuditoriaConfig {
    /// Erro específico do adaptador de persistência.
    type Erro;

    /// Registra uma mudança de configuração na trilha de auditoria.
    ///
    /// # Errors
    /// Retorna `Self::Erro` se a trilha não puder ser gravada.
    fn registrar(&self, mudanca: &MudancaConfig) -> Result<(), Self::Erro>;
}
