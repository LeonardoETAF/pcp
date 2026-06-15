//! Fatores sazonais (`pcp.fatores_sazonais`) e agregação de vendas por mês para o cálculo
//! (doc 02 §4). A regra (fator/clamp) vive no `pcp-core`; aqui só persistência e agregação.

use chrono::NaiveDate;
use sqlx::PgPool;

use crate::erro::ErroDb;

/// Total vendido e dias com venda de um mês (insumo da média diária — doc 02 §4.1).
#[derive(Debug, Clone)]
pub struct VendasMes {
    pub mes: i32,
    pub total: f64,
    pub dias: i64,
}

/// Data da última atualização dos fatores (doc 02 §4.2). `None` se a tabela está vazia.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn ultima_atualizacao(pool: &PgPool) -> Result<Option<NaiveDate>, ErroDb> {
    let data = sqlx::query_scalar!(
        r#"SELECT MAX(atualizado_em)::date AS "data?" FROM pcp.fatores_sazonais"#
    )
    .fetch_one(pool)
    .await?;
    Ok(data)
}

/// Lê os fatores persistidos como pares `(mes, fator)`, ordenados por mês.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn listar(pool: &PgPool) -> Result<Vec<(i16, f64)>, ErroDb> {
    let linhas = sqlx::query!("SELECT mes, fator FROM pcp.fatores_sazonais ORDER BY mes")
        .fetch_all(pool)
        .await?;
    Ok(linhas.into_iter().map(|l| (l.mes, l.fator)).collect())
}

/// Substitui (upsert) os 12 fatores numa transação. Índice 0 = mês 1.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco; a transação é revertida.
pub async fn substituir(pool: &PgPool, fatores: &[f64; 12]) -> Result<(), ErroDb> {
    let mut tx = pool.begin().await?;
    for (indice, &fator) in fatores.iter().enumerate() {
        let mes = i16::try_from(indice + 1).unwrap_or(1);
        sqlx::query!(
            "INSERT INTO pcp.fatores_sazonais (mes, fator, atualizado_em) VALUES ($1, $2, now()) \
             ON CONFLICT (mes) DO UPDATE SET fator = EXCLUDED.fator, atualizado_em = now()",
            mes,
            fator,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Total e dias com venda por mês no intervalo `[inicio, fim)` (doc 02 §4.1 — só dias com
/// venda, como na §3.1). Usado para a média diária do mês e do ano anterior.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn vendas_por_mes(
    pool: &PgPool,
    inicio: NaiveDate,
    fim: NaiveDate,
) -> Result<Vec<VendasMes>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT EXTRACT(MONTH FROM dt_ref)::int4 AS "mes!",
                  SUM(qtd_vendida)::float8        AS "total!",
                  COUNT(DISTINCT dt_ref)          AS "dias!"
           FROM pcp.vendas_dia
           WHERE dt_ref >= $1 AND dt_ref < $2 AND qtd_vendida > 0
           GROUP BY 1
           ORDER BY 1"#,
        inicio,
        fim,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| VendasMes {
            mes: l.mes,
            total: l.total,
            dias: l.dias,
        })
        .collect())
}
