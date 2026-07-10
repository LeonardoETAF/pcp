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

/// Ordens de produção (F06002), com configuração — full refresh. `iprd_cnf` liga a ordem à linha
/// de estoque via (item, configuração); `iprd_qntt` é o quanto já foi produzido.
const SQL_PRODUCAO: &str = "\
SELECT i.iprd_id AS iprd_id, i.iprd_prd AS iprd_prd, i.iprd_cnf AS iprd_cnf, \
       ROUND(i.iprd_qnt)::int AS iprd_qnt, ROUND(COALESCE(i.iprd_qntt, 0))::int AS iprd_qntt, \
       i.iprd_stat AS iprd_stat, i.iprd_lote AS iprd_lote, i.aud_date::date AS aud_date \
FROM prd.f06002 i WHERE i.iprd_prd IS NOT NULL";

/// Kardex (F03007) por linha de estoque — incremental por data. Landa TODOS os tipos de movimento
/// (venda, produção, inventário, ...) para a linha do tempo. `$1` = início da janela.
const SQL_MOVIMENTO: &str = "\
SELECT c.cdx_id AS cdx_id, c.cdx_estq AS cdx_estq, c.cdx_datc::date AS cdx_datc, \
       c.cdx_tpmvm AS cdx_tpmvm, ROUND(c.cdx_qtd)::int AS cdx_qtd, \
       ROUND(c.cdx_sd)::bigint AS cdx_sd \
FROM prd.f03007 c \
JOIN prd.f03005 e ON e.est_id = c.cdx_estq \
JOIN prd.f03001 p ON p.itm_id = e.est_itm \
WHERE p.itm_gpprd = 'PRODUTO_ACABADO' AND NOT COALESCE(p.itm_proda, false) \
  AND BTRIM(COALESCE(e.est_dconf, '')) <> '' AND c.cdx_datc >= $1";

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
    iprd_cnf: Option<i64>,
    iprd_qnt: i32,
    iprd_qntt: i32,
    iprd_stat: Option<String>,
    iprd_lote: Option<i64>,
    aud_date: Option<NaiveDate>,
}

/// Movimento do kardex lido do One.
#[allow(clippy::struct_field_names)] // nomes espelham as colunas do One (cdx_*) de propósito
struct LinhaMovimento {
    cdx_id: i64,
    cdx_estq: i64,
    cdx_datc: NaiveDate,
    cdx_tpmvm: String,
    cdx_qtd: i32,
    cdx_sd: i64,
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
                iprd_cnf: r.try_get::<Option<i64>, _>("iprd_cnf")?,
                iprd_qnt: inteiro(r, "iprd_qnt")?,
                iprd_qntt: inteiro(r, "iprd_qntt")?,
                iprd_stat: texto(r, "iprd_stat")?,
                iprd_lote: r.try_get::<Option<i64>, _>("iprd_lote")?,
                aud_date: r.try_get::<Option<NaiveDate>, _>("aud_date")?,
            })
        })
        .collect::<Result<_, ErroEtl>>()?;
    let mut tx = pcp.begin().await?;
    sqlx::query("TRUNCATE bronze.one_producao")
        .execute(&mut *tx)
        .await?;
    for lote in itens.chunks(LOTE) {
        let mut qb = QueryBuilder::new(
            "INSERT INTO bronze.one_producao \
             (iprd_id, iprd_prd, iprd_cnf, iprd_qnt, iprd_qntt, iprd_stat, iprd_lote, aud_date) ",
        );
        qb.push_values(lote, |mut b, p| {
            b.push_bind(p.iprd_id)
                .push_bind(p.iprd_prd)
                .push_bind(p.iprd_cnf)
                .push_bind(p.iprd_qnt)
                .push_bind(p.iprd_qntt)
                .push_bind(p.iprd_stat.as_deref())
                .push_bind(p.iprd_lote)
                .push_bind(p.aud_date);
        });
        qb.build().execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(itens.len() as u64)
}

/// Sincroniza o kardex (janela deslizante a partir de `desde`) para `bronze.one_movimento`.
/// Retorna o número de linhas gravadas.
///
/// # Errors
/// [`ErroEtl::One`] na consulta ao One ou na gravação do bronze.
pub async fn sincronizar_movimentos(
    one: &PgPool,
    pcp: &PgPool,
    desde: NaiveDate,
) -> Result<u64, ErroEtl> {
    let movs: Vec<LinhaMovimento> = sqlx::query(SQL_MOVIMENTO)
        .bind(desde)
        .fetch_all(one)
        .await?
        .iter()
        .map(|r| {
            Ok(LinhaMovimento {
                cdx_id: r.try_get("cdx_id")?,
                cdx_estq: r.try_get("cdx_estq")?,
                cdx_datc: r.try_get("cdx_datc")?,
                cdx_tpmvm: r.try_get("cdx_tpmvm")?,
                cdx_qtd: inteiro(r, "cdx_qtd")?,
                cdx_sd: r.try_get::<Option<i64>, _>("cdx_sd")?.unwrap_or(0),
            })
        })
        .collect::<Result<_, ErroEtl>>()?;
    let mut tx = pcp.begin().await?;
    sqlx::query("DELETE FROM bronze.one_movimento WHERE cdx_datc >= $1")
        .bind(desde)
        .execute(&mut *tx)
        .await?;
    for lote in movs.chunks(LOTE) {
        let mut qb = QueryBuilder::new(
            "INSERT INTO bronze.one_movimento \
             (cdx_id, cdx_estq, cdx_datc, cdx_tpmvm, cdx_qtd, cdx_sd) ",
        );
        qb.push_values(lote, |mut b, m| {
            b.push_bind(m.cdx_id)
                .push_bind(m.cdx_estq)
                .push_bind(m.cdx_datc)
                .push_bind(m.cdx_tpmvm.as_str())
                .push_bind(m.cdx_qtd)
                .push_bind(m.cdx_sd);
        });
        qb.build().execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(movs.len() as u64)
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
