//! Trait `FonteDados`: a fronteira que desacopla o PCP da origem dos dados (CLAUDE.md §1/§8).
//! O motor só conhece as tabelas de entrada; trocar a fonte (arquivo → ERP "One") não o afeta.

use pcp_db::{NovaVendaDia, NovoEstoqueSnapshot};

use crate::erro::ErroEtl;

/// Fonte de dados de entrada do PCP, **assíncrona** (CLAUDE.md §1/§8). Implementada pelo
/// `ImportadorArquivo` (CSV/dump, backfill) e pelo `FonteConsultaOne` (consulta read-only ao
/// ERP One, incremental). O resto do sistema só conhece este contrato — trocar a fonte não o
/// afeta. A fonte decide internamente o que devolve (tudo no backfill; a janela no incremental).
#[allow(async_fn_in_trait)] // futuros consumidos localmente na ingestão; não exigem `Send` no contrato
pub trait FonteDados {
    /// Lê as vendas disponíveis na fonte (contrato doc 05 §2.1).
    ///
    /// # Errors
    /// [`ErroEtl`] se a leitura ou a validação do contrato falhar.
    async fn ler_vendas(&self) -> Result<Vec<NovaVendaDia>, ErroEtl>;

    /// Lê os snapshots de estoque disponíveis na fonte (contrato doc 05 §2.2).
    ///
    /// # Errors
    /// [`ErroEtl`] se a leitura ou a validação do contrato falhar.
    async fn ler_snapshots(&self) -> Result<Vec<NovoEstoqueSnapshot>, ErroEtl>;
}
