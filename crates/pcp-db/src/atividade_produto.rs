//! Atividade de uma linha de estoque para a tela de detalhe (doc 03 §4): movimentação e produção.
//! Lê o `bronze` (kardex e ordens do One) — são dados de APOIO, só exibição, não entram no motor.
//! Tudo por `est_id` (o `codigo_estoque`): movimento casa direto (`cdx_estq`); produção casa via
//! `(est_itm, est_cnf)` da linha de estoque.

use sqlx::PgPool;

use crate::erro::ErroDb;

/// Um movimento do kardex (entrada/saída), do mais recente ao mais antigo.
#[derive(Debug, Clone)]
pub struct Movimento {
    pub data: chrono::NaiveDate,
    pub tipo: String,
    pub quantidade: i32,
    pub saldo: i64,
}

/// Uma ordem de produção da linha de estoque.
#[derive(Debug, Clone)]
pub struct OrdemProducao {
    pub data: Option<chrono::NaiveDate>,
    pub quantidade: i32,
    pub status: Option<String>,
    pub lote: Option<i64>,
}

/// Situação atual da produção: ordens abertas (aguardando/em produção) e o total planejado nelas.
#[derive(Debug, Clone, Default)]
pub struct StatusProducao {
    pub ordens_abertas: i64,
    pub qtd_planejada: i64,
    pub em_producao: i64,
    pub aguardando: i64,
}

/// Últimos `limite` movimentos do kardex da linha de estoque (mais recentes primeiro).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco (inclui código não-numérico).
pub async fn movimentos(
    pool: &PgPool,
    codigo: &str,
    limite: i64,
) -> Result<Vec<Movimento>, ErroDb> {
    let Ok(est_id) = codigo.parse::<i64>() else {
        return Ok(Vec::new());
    };
    let linhas = sqlx::query!(
        r#"SELECT cdx_datc AS "data!", cdx_tpmvm AS "tipo!", cdx_qtd AS "quantidade!",
                  cdx_sd AS "saldo!"
           FROM bronze.one_movimento
           WHERE cdx_estq = $1
           ORDER BY cdx_datc DESC, cdx_id DESC
           LIMIT $2"#,
        est_id,
        limite,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|r| Movimento {
            data: r.data,
            tipo: r.tipo,
            quantidade: r.quantidade,
            saldo: r.saldo,
        })
        .collect())
}

/// Últimas `limite` ordens de produção da linha (mais recentes primeiro), exceto canceladas.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco (inclui código não-numérico).
pub async fn producao_historico(
    pool: &PgPool,
    codigo: &str,
    limite: i64,
) -> Result<Vec<OrdemProducao>, ErroDb> {
    let Ok(est_id) = codigo.parse::<i64>() else {
        return Ok(Vec::new());
    };
    let linhas = sqlx::query!(
        r#"SELECT p.aud_date AS data, p.iprd_qnt AS "quantidade!", p.iprd_stat AS status,
                  p.iprd_lote AS lote
           FROM bronze.one_producao p
           JOIN bronze.one_estoque e
             ON e.est_itm = p.iprd_prd AND e.est_cnf = p.iprd_cnf
           WHERE e.est_id = $1
             AND e.data_ref = (SELECT MAX(data_ref) FROM bronze.one_estoque)
             AND COALESCE(p.iprd_stat, '') <> 'CANCELADO'
           ORDER BY p.aud_date DESC NULLS LAST, p.iprd_id DESC
           LIMIT $2"#,
        est_id,
        limite,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|r| OrdemProducao {
            data: r.data,
            quantidade: r.quantidade,
            status: r.status,
            lote: r.lote,
        })
        .collect())
}

/// Situação atual da produção da linha: ordens abertas (AGUARDANDO/PRODUCAO) e o total planejado.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco (inclui código não-numérico).
pub async fn status_producao(pool: &PgPool, codigo: &str) -> Result<StatusProducao, ErroDb> {
    let Ok(est_id) = codigo.parse::<i64>() else {
        return Ok(StatusProducao::default());
    };
    let r = sqlx::query!(
        r#"SELECT
             COUNT(*) FILTER (WHERE p.iprd_stat IN ('AGUARDANDO', 'PRODUCAO')) AS "abertas!",
             COALESCE(SUM(p.iprd_qnt) FILTER (WHERE p.iprd_stat IN ('AGUARDANDO', 'PRODUCAO')), 0)
               AS "planejada!",
             COUNT(*) FILTER (WHERE p.iprd_stat = 'PRODUCAO') AS "em_producao!",
             COUNT(*) FILTER (WHERE p.iprd_stat = 'AGUARDANDO') AS "aguardando!"
           FROM bronze.one_producao p
           JOIN bronze.one_estoque e
             ON e.est_itm = p.iprd_prd AND e.est_cnf = p.iprd_cnf
           WHERE e.est_id = $1
             AND e.data_ref = (SELECT MAX(data_ref) FROM bronze.one_estoque)"#,
        est_id,
    )
    .fetch_one(pool)
    .await?;
    Ok(StatusProducao {
        ordens_abertas: r.abertas,
        qtd_planejada: r.planejada,
        em_producao: r.em_producao,
        aguardando: r.aguardando,
    })
}
