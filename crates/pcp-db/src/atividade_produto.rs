//! Atividade de uma linha de estoque para a tela de detalhe (doc 03 §4): movimentação e produção.
//! Lê o `bronze` (kardex e ordens do One) — são dados de APOIO, só exibição, não entram no motor.
//! Movimento casa direto por `est_id` (`cdx_estq`). Produção casa pelo ITEM (`est_itm = iprd_prd`)
//! — a ordem em produção tem `iprd_cnf` nulo (produz-se o liso; a cor entra depois), então o
//! histórico e o status são do item, comuns às cores dele.

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
    pub produzido: i32,
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
    /// Planejado e produzido SOMENTE das ordens em produção (para "quanto falta").
    pub planejado_em_producao: i64,
    pub produzido_em_producao: i64,
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
        r#"SELECT p.aud_date AS data, p.iprd_qnt AS "quantidade!", p.iprd_qntt AS "produzido!",
                  p.iprd_stat AS status, p.iprd_lote AS lote
           FROM bronze.one_producao p
           WHERE p.iprd_prd = (SELECT est_itm FROM bronze.one_estoque
                               WHERE est_id = $1
                                 AND data_ref = (SELECT MAX(data_ref) FROM bronze.one_estoque)
                               LIMIT 1)
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
            produzido: r.produzido,
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
             COUNT(*) FILTER (WHERE p.iprd_stat = 'AGUARDANDO') AS "aguardando!",
             COALESCE(SUM(p.iprd_qnt) FILTER (WHERE p.iprd_stat = 'PRODUCAO'), 0) AS "plan_prod!",
             COALESCE(SUM(p.iprd_qntt) FILTER (WHERE p.iprd_stat = 'PRODUCAO'), 0) AS "prod_prod!"
           FROM bronze.one_producao p
           WHERE p.iprd_prd = (SELECT est_itm FROM bronze.one_estoque
                               WHERE est_id = $1
                                 AND data_ref = (SELECT MAX(data_ref) FROM bronze.one_estoque)
                               LIMIT 1)"#,
        est_id,
    )
    .fetch_one(pool)
    .await?;
    Ok(StatusProducao {
        ordens_abertas: r.abertas,
        qtd_planejada: r.planejada,
        em_producao: r.em_producao,
        aguardando: r.aguardando,
        planejado_em_producao: r.plan_prod,
        produzido_em_producao: r.prod_prod,
    })
}
