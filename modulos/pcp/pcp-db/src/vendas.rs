//! Repositório das vendas diárias (`pcp.vendas_dia`). Idempotência por dia. Sem regra.

use chrono::NaiveDate;
use sqlx::{PgPool, QueryBuilder};

use crate::erro::ErroDb;
use crate::modelos::{NovaVendaDia, VendaDia};

/// Linhas por lote no INSERT em batch (folga sobre o teto de parâmetros do Postgres).
const LOTE: usize = 5_000;

/// Substitui de forma idempotente as vendas de `dt_ref`: numa única transação, apaga as do
/// dia e insere as novas EM LOTE (CLAUDE.md §3.3/§6/§15 — sem INSERT linha-a-linha). Retorna a
/// quantidade de linhas inseridas.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco; a transação é revertida sem efeito colateral.
pub async fn substituir_dia(
    pool: &PgPool,
    dt_ref: NaiveDate,
    vendas: &[NovaVendaDia],
) -> Result<u64, ErroDb> {
    let mut tx = pool.begin().await?;
    sqlx::query!("DELETE FROM pcp.vendas_dia WHERE dt_ref = $1", dt_ref)
        .execute(&mut *tx)
        .await?;
    let mut inseridas: u64 = 0;
    for lote in vendas.chunks(LOTE) {
        let mut qb = QueryBuilder::new(
            "INSERT INTO pcp.vendas_dia \
             (dt_ref, codigo_estoque, sku, produto, configuracao, qtd_vendida, is_personalizado) ",
        );
        qb.push_values(lote, |mut b, v| {
            b.push_bind(v.dt_ref)
                .push_bind(&v.codigo_estoque)
                .push_bind(v.sku.as_deref())
                .push_bind(v.produto.as_deref())
                .push_bind(v.configuracao.as_deref())
                .push_bind(v.qtd_vendida)
                .push_bind(v.is_personalizado);
        });
        inseridas += qb.build().execute(&mut *tx).await?.rows_affected();
    }
    tx.commit().await?;
    Ok(inseridas)
}

/// Lê as vendas de uma data, em ordem estável (por código e id).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn do_dia(pool: &PgPool, dt_ref: NaiveDate) -> Result<Vec<VendaDia>, ErroDb> {
    let linhas = sqlx::query_as!(
        VendaDia,
        "SELECT id, dt_ref, codigo_estoque, sku, produto, configuracao, qtd_vendida, \
         is_personalizado, ingerido_em \
         FROM pcp.vendas_dia WHERE dt_ref = $1 ORDER BY codigo_estoque, id",
        dt_ref,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas)
}

/// Conta as linhas de venda de uma data (apoio à pré-validação do pipeline — doc 05 §3).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn contar_do_dia(pool: &PgPool, dt_ref: NaiveDate) -> Result<i64, ErroDb> {
    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pcp.vendas_dia WHERE dt_ref = $1",
        dt_ref,
    )
    .fetch_one(pool)
    .await?;
    Ok(total.unwrap_or(0))
}
