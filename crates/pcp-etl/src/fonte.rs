//! Trait `FonteDados`: a fronteira que desacopla o PCP da origem dos dados (CLAUDE.md §1/§8).
//! O motor só conhece as tabelas de entrada; trocar a fonte (arquivo → ERP "One") não o afeta.

use pcp_db::{NovaVendaDia, NovoEstoqueSnapshot};

use crate::erro::ErroEtl;

/// Fonte de dados de entrada do PCP. Hoje implementada por `ImportadorArquivo` (CSV/dump);
/// amanhã por um conector ao ERP "One", sem alterar o resto do sistema.
pub trait FonteDados {
    /// Lê todas as vendas disponíveis na fonte (contrato doc 05 §2.1).
    ///
    /// # Errors
    /// [`ErroEtl`] se a leitura ou a validação do contrato falhar.
    fn ler_vendas(&self) -> Result<Vec<NovaVendaDia>, ErroEtl>;

    /// Lê todos os snapshots de estoque disponíveis na fonte (contrato doc 05 §2.2).
    ///
    /// # Errors
    /// [`ErroEtl`] se a leitura ou a validação do contrato falhar.
    fn ler_snapshots(&self) -> Result<Vec<NovoEstoqueSnapshot>, ErroEtl>;
}
