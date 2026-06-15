//! Agregações de leitura para o motor diário (doc 05 §1.2). A agregação pesada roda no banco
//! (CLAUDE.md §15); o pcp-core só aplica a regra sobre os agregados.

use chrono::NaiveDate;
use sqlx::PgPool;

use crate::erro::ErroDb;

/// Insumos por produto para a classificação e a análise de fora de linha (doc 02 §2/§8),
/// a partir do snapshot de `data_ref` cruzado com agregados de venda.
#[derive(Debug, Clone)]
pub struct BaseProduto {
    pub codigo_estoque: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub fora_de_linha: bool,
    pub qtd_estoque: i32,
    pub qtd_reserva: i32,
    pub qtd_disponivel: i32,
    pub primeira_venda: Option<NaiveDate>,
    pub ultima_venda: Option<NaiveDate>,
    /// Volume somado na janela ABC (doc 02 §2.4).
    pub volume_540: i64,
    /// Unidades vendidas nos últimos 365 dias (doc 02 §8.1).
    pub vendas_365: i64,
}

/// Conta as linhas de venda de uma data (pré-validação — doc 05 §3).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn contar_vendas(pool: &PgPool, dt_ref: NaiveDate) -> Result<i64, ErroDb> {
    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pcp.vendas_dia WHERE dt_ref = $1",
        dt_ref
    )
    .fetch_one(pool)
    .await?;
    Ok(total.unwrap_or(0))
}

/// Conta as linhas do snapshot de uma data (pré-validação — doc 05 §3).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn contar_snapshot(pool: &PgPool, dt_ref: NaiveDate) -> Result<i64, ErroDb> {
    let total = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pcp.estoque_snapshot WHERE dt_ref = $1",
        dt_ref
    )
    .fetch_one(pool)
    .await?;
    Ok(total.unwrap_or(0))
}

/// Insumos por produto a partir do snapshot de `data_ref` + agregados de venda.
/// `inicio_540`/`inicio_365` são os limites inferiores (exclusivos) das janelas.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn base_produtos(
    pool: &PgPool,
    data_ref: NaiveDate,
    inicio_540: NaiveDate,
    inicio_365: NaiveDate,
) -> Result<Vec<BaseProduto>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT s.codigo_estoque       AS "codigo_estoque!",
                  s.sku                  AS "sku?",
                  s.produto              AS "produto?",
                  s.configuracao         AS "configuracao?",
                  s.fora_de_linha        AS "fora_de_linha!",
                  s.qtd_estoque          AS "qtd_estoque!",
                  s.qtd_reserva          AS "qtd_reserva!",
                  s.qtd_disponivel       AS "qtd_disponivel!",
                  v.primeira_venda       AS "primeira_venda?",
                  v.ultima_venda         AS "ultima_venda?",
                  COALESCE(v.volume_540, 0)::bigint AS "volume_540!",
                  COALESCE(v.vendas_365, 0)::bigint AS "vendas_365!"
           FROM pcp.estoque_snapshot s
           LEFT JOIN (
               SELECT codigo_estoque,
                      MIN(dt_ref) AS primeira_venda,
                      MAX(dt_ref) AS ultima_venda,
                      SUM(qtd_vendida) FILTER (WHERE dt_ref > $2) AS volume_540,
                      SUM(qtd_vendida) FILTER (WHERE dt_ref > $3) AS vendas_365
               FROM pcp.vendas_dia
               WHERE dt_ref <= $1
               GROUP BY codigo_estoque
           ) v ON v.codigo_estoque = s.codigo_estoque
           WHERE s.dt_ref = $1
           ORDER BY s.codigo_estoque"#,
        data_ref,
        inicio_540,
        inicio_365,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| BaseProduto {
            codigo_estoque: l.codigo_estoque,
            sku: l.sku,
            produto: l.produto,
            configuracao: l.configuracao,
            fora_de_linha: l.fora_de_linha,
            qtd_estoque: l.qtd_estoque,
            qtd_reserva: l.qtd_reserva,
            qtd_disponivel: l.qtd_disponivel,
            primeira_venda: l.primeira_venda,
            ultima_venda: l.ultima_venda,
            volume_540: l.volume_540,
            vendas_365: l.vendas_365,
        })
        .collect())
}

/// Quantidade vendida por (produto, dia) na janela `(inicio_365, data_ref]`, só dias com
/// venda (doc 02 §3.1). O chamador agrupa por produto para os parâmetros estatísticos.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn vendas_diarias(
    pool: &PgPool,
    data_ref: NaiveDate,
    inicio_365: NaiveDate,
) -> Result<Vec<(String, i64)>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT codigo_estoque AS "codigo!", SUM(qtd_vendida)::bigint AS "qtd!"
           FROM pcp.vendas_dia
           WHERE dt_ref > $2 AND dt_ref <= $1 AND qtd_vendida > 0
           GROUP BY codigo_estoque, dt_ref
           ORDER BY codigo_estoque"#,
        data_ref,
        inicio_365,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas.into_iter().map(|l| (l.codigo, l.qtd)).collect())
}
