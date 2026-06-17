//! Leitura do Detalhe do Produto (doc 03 §4 / doc 04 §6.2). Junta o snapshot calculado pelo
//! motor (`produto_ativo`) com a posição na classificação (`classificacao`, p/ Pareto) e a
//! qualidade dos dados (`estoque_param`, p/ outliers). Séries de 90 dias de vendas e estoque para
//! os gráficos. **Nenhuma regra é recalculada aqui** (CLAUDE.md §3.2).

use chrono::NaiveDate;
use sqlx::PgPool;

use crate::erro::ErroDb;

/// Detalhe completo de um produto (valores já calculados pelo motor).
#[derive(Debug, Clone)]
pub struct DetalheProduto {
    pub codigo_estoque: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub classe: String,
    pub status: String,
    pub fora_de_linha: bool,
    pub fator_estoque: f64,
    pub percentual_acumulado: Option<f64>,
    pub qtd_estoque: i64,
    pub qtd_reserva: i64,
    pub qtd_disponivel: i64,
    pub cobertura_dias: f64,
    pub media_diaria: f64,
    pub estoque_seguranca: i64,
    pub estoque_minimo: i64,
    pub estoque_total_recomendado: i64,
    pub qtd_sugerida: i64,
    pub volume_janela: i64,
    pub dias_com_vendas: i64,
    pub outliers_detectados: i64,
    pub coef_variacao: f64,
    pub fator_sazonal: f64,
    pub dt_ref: NaiveDate,
}

/// Um ponto de série temporal (dia → valor) para os gráficos de 90 dias.
#[derive(Debug, Clone)]
pub struct PontoSerie {
    pub data: NaiveDate,
    pub valor: i64,
}

/// Detalhe de um produto pelo código. `None` se não existir em `produto_ativo`.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn produto(pool: &PgPool, codigo: &str) -> Result<Option<DetalheProduto>, ErroDb> {
    let linha = sqlx::query!(
        r#"SELECT p.codigo_estoque, p.sku, p.produto, p.configuracao, p.classe, p.status,
                  p.fora_de_linha, p.fator_estoque, p.qtd_estoque, p.qtd_reserva,
                  p.qtd_disponivel, p.cobertura_dias, p.media_diaria, p.estoque_seguranca,
                  p.estoque_minimo, p.estoque_total_recomendado, p.qtd_sugerida,
                  p.volume_janela, p.dias_com_vendas, p.coef_variacao, p.dt_ref,
                  COALESCE(ep.outliers_detectados, 0)::bigint AS "outliers_detectados!",
                  COALESCE(ep.fator_sazonal, 1.0)::float8 AS "fator_sazonal!",
                  c.percentual_acumulado AS "percentual_acumulado?"
           FROM pcp.produto_ativo p
           LEFT JOIN pcp.estoque_param ep ON ep.codigo_estoque = p.codigo_estoque
           LEFT JOIN pcp.classificacao c ON c.codigo_estoque = p.codigo_estoque
                AND c.dt_calculo = (SELECT MAX(dt_calculo) FROM pcp.classificacao
                                    WHERE codigo_estoque = p.codigo_estoque)
           WHERE p.codigo_estoque = $1"#,
        codigo,
    )
    .fetch_optional(pool)
    .await?;

    Ok(linha.map(|r| DetalheProduto {
        codigo_estoque: r.codigo_estoque,
        sku: r.sku,
        produto: r.produto,
        configuracao: r.configuracao,
        classe: r.classe,
        status: r.status,
        fora_de_linha: r.fora_de_linha,
        fator_estoque: r.fator_estoque,
        percentual_acumulado: r.percentual_acumulado,
        qtd_estoque: r.qtd_estoque,
        qtd_reserva: r.qtd_reserva,
        qtd_disponivel: r.qtd_disponivel,
        cobertura_dias: r.cobertura_dias,
        media_diaria: r.media_diaria,
        estoque_seguranca: r.estoque_seguranca,
        estoque_minimo: r.estoque_minimo,
        estoque_total_recomendado: r.estoque_total_recomendado,
        qtd_sugerida: r.qtd_sugerida,
        volume_janela: r.volume_janela,
        dias_com_vendas: r.dias_com_vendas,
        outliers_detectados: r.outliers_detectados,
        coef_variacao: r.coef_variacao,
        fator_sazonal: r.fator_sazonal,
        dt_ref: r.dt_ref,
    }))
}

/// Vendas diárias (soma por dia) dos 90 dias até `ate` (inclusive) — gráfico do doc 03 §4.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn vendas_90d(
    pool: &PgPool,
    codigo: &str,
    ate: NaiveDate,
) -> Result<Vec<PontoSerie>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT dt_ref AS "data!", SUM(qtd_vendida)::bigint AS "valor!"
           FROM pcp.vendas_dia
           WHERE codigo_estoque = $1
             AND dt_ref > ($2::date - INTERVAL '90 days') AND dt_ref <= $2
           GROUP BY dt_ref ORDER BY dt_ref"#,
        codigo,
        ate,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|r| PontoSerie {
            data: r.data,
            valor: r.valor,
        })
        .collect())
}

/// Série de vendas (soma por dia) no intervalo `[inicio, ate]` — insumo dos insights (doc 06 §3).
/// Só dias com venda; o motor de insights densifica os dias sem registro.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn vendas_intervalo(
    pool: &PgPool,
    codigo: &str,
    inicio: NaiveDate,
    ate: NaiveDate,
) -> Result<Vec<PontoSerie>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT dt_ref AS "data!", SUM(qtd_vendida)::bigint AS "valor!"
           FROM pcp.vendas_dia
           WHERE codigo_estoque = $1 AND dt_ref >= $2 AND dt_ref <= $3
           GROUP BY dt_ref ORDER BY dt_ref"#,
        codigo,
        inicio,
        ate,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|r| PontoSerie {
            data: r.data,
            valor: r.valor,
        })
        .collect())
}

/// Evolução do estoque disponível (snapshot diário) dos 90 dias até `ate` — gráfico do doc 03 §4.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn estoque_90d(
    pool: &PgPool,
    codigo: &str,
    ate: NaiveDate,
) -> Result<Vec<PontoSerie>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT dt_ref AS "data!", qtd_disponivel::bigint AS "valor!"
           FROM pcp.estoque_snapshot
           WHERE codigo_estoque = $1
             AND dt_ref > ($2::date - INTERVAL '90 days') AND dt_ref <= $2
           ORDER BY dt_ref"#,
        codigo,
        ate,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|r| PontoSerie {
            data: r.data,
            valor: r.valor,
        })
        .collect())
}
