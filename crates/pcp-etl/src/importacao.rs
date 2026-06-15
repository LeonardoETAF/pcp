//! Importação para as tabelas de entrada, idempotente por dia (doc 05 §2.3).

use std::collections::BTreeMap;

use chrono::NaiveDate;

use pcp_db::{NovaVendaDia, NovoEstoqueSnapshot, PgPool};

use crate::erro::ErroEtl;
use crate::fonte::FonteDados;

/// Resumo de uma importação (dias e linhas gravados).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResumoImportacao {
    pub dias_vendas: usize,
    pub linhas_vendas: u64,
    pub dias_snapshot: usize,
    pub linhas_snapshot: u64,
}

/// Lê de uma `FonteDados` e grava nas tabelas de entrada, substituindo por dia (idempotente —
/// doc 05 §2.3): reprocessar uma data troca os dados daquele dia sem efeito colateral.
///
/// # Errors
/// [`ErroEtl`] se a leitura, a validação do contrato ou a gravação falharem.
pub async fn importar<F: FonteDados>(
    pool: &PgPool,
    fonte: &F,
) -> Result<ResumoImportacao, ErroEtl> {
    let vendas = fonte.ler_vendas()?;
    let snapshots = fonte.ler_snapshots()?;

    let por_dia_vendas = agrupar_por_dia(vendas, |v: &NovaVendaDia| v.dt_ref);
    let mut resumo = ResumoImportacao {
        dias_vendas: por_dia_vendas.len(),
        ..Default::default()
    };
    for (dia, linhas) in &por_dia_vendas {
        resumo.linhas_vendas += pcp_db::vendas::substituir_dia(pool, *dia, linhas).await?;
    }

    let por_dia_snapshot = agrupar_por_dia(snapshots, |s: &NovoEstoqueSnapshot| s.dt_ref);
    resumo.dias_snapshot = por_dia_snapshot.len();
    for (dia, linhas) in &por_dia_snapshot {
        resumo.linhas_snapshot += pcp_db::snapshot::substituir_dia(pool, *dia, linhas).await?;
    }

    Ok(resumo)
}

/// Agrupa itens por `dt_ref` em ordem cronológica (determinístico).
fn agrupar_por_dia<T>(
    itens: Vec<T>,
    data: impl Fn(&T) -> NaiveDate,
) -> BTreeMap<NaiveDate, Vec<T>> {
    let mut mapa: BTreeMap<NaiveDate, Vec<T>> = BTreeMap::new();
    for item in itens {
        mapa.entry(data(&item)).or_default().push(item);
    }
    mapa
}
