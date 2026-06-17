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

/// Linha de fatura lida do One (tipada — erro de coluna propaga, não vira valor silencioso).
struct LinhaFatura {
    fat_dtemi: NaiveDate,
    fti_prod: i64,
    fti_dprd: Option<String>,
    fti_qtde: i32,
}

/// Item de produção lido do One.
#[allow(clippy::struct_field_names)] // nomes espelham as colunas do One (iprd_*) de propósito
struct LinhaProducao {
    iprd_id: i64,
    iprd_prd: i64,
    iprd_qnt: i32,
    iprd_stat: Option<String>,
}

/// Sincroniza as faturas (janela deslizante a partir de `desde`) para `bronze.one_fatura`.
/// Retorna o número de linhas gravadas.
///
/// # Errors
/// [`ErroEtl::One`] na consulta ao One ou na gravação do bronze.
pub async fn sincronizar_faturas(
    one: &PgPool,
    pcp: &PgPool,
    desde: NaiveDate,
) -> Result<u64, ErroEtl> {
    let faturas: Vec<LinhaFatura> = sqlx::query(SQL_FATURA)
        .bind(desde)
        .fetch_all(one)
        .await?
        .iter()
        .map(|r| {
            Ok(LinhaFatura {
                fat_dtemi: r.try_get("fat_dtemi")?,
                fti_prod: r.try_get("fti_prod")?,
                fti_dprd: texto(r, "fti_dprd")?,
                fti_qtde: inteiro(r, "fti_qtde")?,
            })
        })
        .collect::<Result<_, ErroEtl>>()?;
    let mut tx = pcp.begin().await?;
    sqlx::query("DELETE FROM bronze.one_fatura WHERE fat_dtemi >= $1")
        .bind(desde)
        .execute(&mut *tx)
        .await?;
    for lote in faturas.chunks(LOTE) {
        let mut qb = QueryBuilder::new(
            "INSERT INTO bronze.one_fatura (fat_dtemi, fti_prod, fti_dprd, fti_qtde) ",
        );
        qb.push_values(lote, |mut b, f| {
            b.push_bind(f.fat_dtemi)
                .push_bind(f.fti_prod)
                .push_bind(f.fti_dprd.as_deref())
                .push_bind(f.fti_qtde);
        });
        qb.build().execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(faturas.len() as u64)
}

/// Sincroniza os itens de produção (full refresh) para `bronze.one_producao`.
/// Retorna o número de linhas gravadas.
///
/// # Errors
/// [`ErroEtl::One`] na consulta ou na gravação do bronze.
pub async fn sincronizar_producao(one: &PgPool, pcp: &PgPool) -> Result<u64, ErroEtl> {
    let itens: Vec<LinhaProducao> = sqlx::query(SQL_PRODUCAO)
        .fetch_all(one)
        .await?
        .iter()
        .map(|r| {
            Ok(LinhaProducao {
                iprd_id: r.try_get("iprd_id")?,
                iprd_prd: r.try_get("iprd_prd")?,
                iprd_qnt: inteiro(r, "iprd_qnt")?,
                iprd_stat: texto(r, "iprd_stat")?,
            })
        })
        .collect::<Result<_, ErroEtl>>()?;
    let mut tx = pcp.begin().await?;
    sqlx::query("TRUNCATE bronze.one_producao")
        .execute(&mut *tx)
        .await?;
    for lote in itens.chunks(LOTE) {
        let mut qb = QueryBuilder::new(
            "INSERT INTO bronze.one_producao (iprd_id, iprd_prd, iprd_qnt, iprd_stat) ",
        );
        qb.push_values(lote, |mut b, p| {
            b.push_bind(p.iprd_id)
                .push_bind(p.iprd_prd)
                .push_bind(p.iprd_qnt)
                .push_bind(p.iprd_stat.as_deref());
        });
        qb.build().execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(itens.len() as u64)
}

/// Lê texto opcional, normalizando branco → `None`; erro de decodificação propaga.
fn texto(r: &PgRow, col: &str) -> Result<Option<String>, ErroEtl> {
    Ok(r.try_get::<Option<String>, _>(col)?
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty()))
}

/// Lê inteiro agregado; `NULL` vira `0`, erro de tipo propaga.
fn inteiro(r: &PgRow, col: &str) -> Result<i32, ErroEtl> {
    Ok(r.try_get::<Option<i32>, _>(col)?.unwrap_or(0))
}
