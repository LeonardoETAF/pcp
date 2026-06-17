//! Fontes COMPLEMENTARES do One (mapeamento §10): venda **faturada** (F10901+F10911) e
//! **produção** em andamento (F06002). NÃO alimentam a demanda (esta vem dos pedidos — doc 02);
//! ficam no `bronze` para visibilidade e uso futuro do motor (abater produção em andamento da
//! sugestão — backlog doc 08 §5). Landing puro (sem ACL p/ domínio): são dados de apoio.

use chrono::NaiveDate;
use sqlx::postgres::{PgPool, PgRow};
use sqlx::{QueryBuilder, Row};

use crate::erro::ErroEtl;

/// Faturas não canceladas, consolidadas por (emissão, produto). `$1` = início da janela.
/// `fti_dprd` (descrição) é denormalizada na fatura e pode variar entre itens do mesmo produto,
/// então agrupamos só por (data, produto) e tomamos `MAX(fti_dprd)` — PK de `one_fatura` é a dupla.
const SQL_FATURA: &str = "\
SELECT c.fat_dtemi::date AS fat_dtemi, i.fti_prod AS fti_prod, MAX(i.fti_dprd) AS fti_dprd, \
       ROUND(SUM(i.fti_qtde))::int AS fti_qtde \
FROM prd.f10911 i JOIN prd.f10901 c ON c.fat_id = i.fti_fatura \
WHERE c.fat_dtcanc IS NULL AND c.fat_stfat NOT IN ('CANCELADA', 'NOTA_FISCAL_CANCEL') \
  AND c.fat_dtemi >= $1 AND i.fti_prod IS NOT NULL \
GROUP BY c.fat_dtemi::date, i.fti_prod";

/// Itens de produção (F06002) — saldo atual por item de produção (full refresh).
const SQL_PRODUCAO: &str = "\
SELECT i.iprd_id AS iprd_id, i.iprd_prd AS iprd_prd, \
       ROUND(i.iprd_qnt)::int AS iprd_qnt, i.iprd_stat AS iprd_stat \
FROM prd.f06002 i WHERE i.iprd_prd IS NOT NULL";

const LOTE: usize = 5_000;

/// Sincroniza as faturas (janela deslizante a partir de `desde`) para `bronze.one_fatura`.
/// Retorna o número de linhas gravadas.
///
/// # Errors
/// [`ErroEtl::One`] na consulta ao One ou [`ErroEtl::Db`]/[`ErroEtl::One`] na gravação do bronze.
pub async fn sincronizar_faturas(
    one: &PgPool,
    pcp: &PgPool,
    desde: NaiveDate,
) -> Result<u64, ErroEtl> {
    let linhas = sqlx::query(SQL_FATURA).bind(desde).fetch_all(one).await?;
    let mut tx = pcp.begin().await?;
    sqlx::query("DELETE FROM bronze.one_fatura WHERE fat_dtemi >= $1")
        .bind(desde)
        .execute(&mut *tx)
        .await?;
    for lote in linhas.chunks(LOTE) {
        let mut qb = QueryBuilder::new(
            "INSERT INTO bronze.one_fatura (fat_dtemi, fti_prod, fti_dprd, fti_qtde) ",
        );
        qb.push_values(lote, |mut b, r| {
            b.push_bind(r.get::<NaiveDate, _>("fat_dtemi"))
                .push_bind(r.get::<i64, _>("fti_prod"))
                .push_bind(texto(r, "fti_dprd"))
                .push_bind(inteiro(r, "fti_qtde"));
        });
        qb.build().execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(linhas.len() as u64)
}

/// Sincroniza os itens de produção (full refresh) para `bronze.one_producao`.
/// Retorna o número de linhas gravadas.
///
/// # Errors
/// [`ErroEtl::One`] na consulta ou na gravação do bronze.
pub async fn sincronizar_producao(one: &PgPool, pcp: &PgPool) -> Result<u64, ErroEtl> {
    let linhas = sqlx::query(SQL_PRODUCAO).fetch_all(one).await?;
    let mut tx = pcp.begin().await?;
    sqlx::query("TRUNCATE bronze.one_producao")
        .execute(&mut *tx)
        .await?;
    for lote in linhas.chunks(LOTE) {
        let mut qb = QueryBuilder::new(
            "INSERT INTO bronze.one_producao (iprd_id, iprd_prd, iprd_qnt, iprd_stat) ",
        );
        qb.push_values(lote, |mut b, r| {
            b.push_bind(r.get::<i64, _>("iprd_id"))
                .push_bind(r.get::<i64, _>("iprd_prd"))
                .push_bind(inteiro(r, "iprd_qnt"))
                .push_bind(texto(r, "iprd_stat"));
        });
        qb.build().execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(linhas.len() as u64)
}

fn texto(r: &PgRow, col: &str) -> Option<String> {
    r.try_get::<Option<String>, _>(col)
        .ok()
        .flatten()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
}

fn inteiro(r: &PgRow, col: &str) -> i32 {
    r.try_get::<Option<i32>, _>(col).ok().flatten().unwrap_or(0)
}
