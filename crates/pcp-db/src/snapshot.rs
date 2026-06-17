//! Repositório do snapshot de estoque (`pcp.estoque_snapshot`). Idempotência por dia.

use chrono::NaiveDate;
use sqlx::{PgPool, QueryBuilder};

use crate::erro::ErroDb;
use crate::modelos::{EstoqueSnapshot, NovoEstoqueSnapshot};

/// Linhas por lote no INSERT em batch (folga sobre o teto de parâmetros do Postgres).
const LOTE: usize = 5_000;

/// Substitui de forma idempotente o snapshot de `dt_ref`: numa única transação, apaga o do
/// dia e insere o novo EM LOTE (snapshot completo — CLAUDE.md §6/§15, sem INSERT linha-a-linha).
/// Retorna as linhas inseridas.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco; a transação é revertida sem efeito colateral.
pub async fn substituir_dia(
    pool: &PgPool,
    dt_ref: NaiveDate,
    snapshot: &[NovoEstoqueSnapshot],
) -> Result<u64, ErroDb> {
    let mut tx = pool.begin().await?;
    sqlx::query!("DELETE FROM pcp.estoque_snapshot WHERE dt_ref = $1", dt_ref)
        .execute(&mut *tx)
        .await?;
    let mut inseridos: u64 = 0;
    for lote in snapshot.chunks(LOTE) {
        let mut qb = QueryBuilder::new(
            "INSERT INTO pcp.estoque_snapshot \
             (dt_ref, codigo_estoque, sku, produto, configuracao, qtd_estoque, qtd_reserva, \
              qtd_disponivel, estoque_min_erp, fora_de_linha) ",
        );
        qb.push_values(lote, |mut b, s| {
            b.push_bind(s.dt_ref)
                .push_bind(&s.codigo_estoque)
                .push_bind(s.sku.as_deref())
                .push_bind(s.produto.as_deref())
                .push_bind(s.configuracao.as_deref())
                .push_bind(s.qtd_estoque)
                .push_bind(s.qtd_reserva)
                .push_bind(s.qtd_disponivel)
                .push_bind(s.estoque_min_erp)
                .push_bind(s.fora_de_linha);
        });
        inseridos += qb.build().execute(&mut *tx).await?.rows_affected();
    }
    tx.commit().await?;
    Ok(inseridos)
}

/// Lê o snapshot de uma data, ordenado por código.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn do_dia(pool: &PgPool, dt_ref: NaiveDate) -> Result<Vec<EstoqueSnapshot>, ErroDb> {
    let linhas = sqlx::query_as!(
        EstoqueSnapshot,
        "SELECT dt_ref, codigo_estoque, sku, produto, configuracao, qtd_estoque, qtd_reserva, \
         qtd_disponivel, estoque_min_erp, fora_de_linha, ingerido_em \
         FROM pcp.estoque_snapshot WHERE dt_ref = $1 ORDER BY codigo_estoque",
        dt_ref,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas)
}

/// Data do snapshot mais recente (`MAX(dt_ref)`), ou `None` se não há snapshots.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn data_mais_recente(pool: &PgPool) -> Result<Option<NaiveDate>, ErroDb> {
    let data = sqlx::query_scalar!("SELECT MAX(dt_ref) FROM pcp.estoque_snapshot")
        .fetch_one(pool)
        .await?;
    Ok(data)
}
