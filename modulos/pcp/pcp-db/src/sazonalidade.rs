//! Fatores sazonais (`pcp.fatores_sazonais`) e agregação de vendas por mês para o cálculo
//! (doc 02 §4). A regra (fator/clamp) vive no `pcp-core`; aqui só persistência e agregação.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::erro::ErroDb;

/// Uma entrada da auditoria de override manual de fator sazonal (doc 02 §4 / §7.5).
#[derive(Debug, Clone)]
pub struct EntradaSazonalAuditoria {
    pub mes: i16,
    pub fator_anterior: Option<f64>,
    pub fator_novo: f64,
    pub justificativa: Option<String>,
    pub por_id: Uuid,
    pub em: DateTime<Utc>,
}

/// Override manual do fator de um mês (gestor), registrando a auditoria, numa transação (§7.5).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn override_mes(
    pool: &PgPool,
    mes: i16,
    fator: f64,
    justificativa: Option<&str>,
    por_id: Uuid,
) -> Result<(), ErroDb> {
    let mut tx = pool.begin().await?;
    let anterior =
        sqlx::query_scalar!("SELECT fator FROM pcp.fatores_sazonais WHERE mes = $1", mes)
            .fetch_optional(&mut *tx)
            .await?;
    sqlx::query!(
        "INSERT INTO pcp.fatores_sazonais (mes, fator) VALUES ($1, $2) \
         ON CONFLICT (mes) DO UPDATE SET fator = $2, atualizado_em = now()",
        mes,
        fator,
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query!(
        "INSERT INTO pcp.fator_sazonal_auditoria (mes, fator_anterior, fator_novo, justificativa, por_id) \
         VALUES ($1, $2, $3, $4, $5)",
        mes,
        anterior,
        fator,
        justificativa,
        por_id,
    )
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// Últimos overrides de sazonalidade (mais recentes primeiro).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn auditoria(pool: &PgPool, limite: i64) -> Result<Vec<EntradaSazonalAuditoria>, ErroDb> {
    let linhas = sqlx::query_as!(
        EntradaSazonalAuditoria,
        "SELECT mes, fator_anterior, fator_novo, justificativa, por_id, em \
         FROM pcp.fator_sazonal_auditoria ORDER BY em DESC LIMIT $1",
        limite,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas)
}

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
/// `calculado_em` é a **data de negócio** do recálculo (a `data_ref` do pipeline), não o relógio:
/// é ela que o gatilho mensal do §4.2 lê de volta em [`ultima_atualizacao`]. Gravar `now()` aqui
/// misturaria relógio real com data de negócio e quebraria a regra sempre que as duas divergissem
/// (reprocesso de uma data passada, por exemplo) — CLAUDE.md §5: o tempo entra por parâmetro.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco; a transação é revertida.
pub async fn substituir(
    pool: &PgPool,
    fatores: &[f64; 12],
    calculado_em: NaiveDate,
) -> Result<(), ErroDb> {
    let mut tx = pool.begin().await?;
    for (indice, &fator) in fatores.iter().enumerate() {
        let mes = i16::try_from(indice + 1).unwrap_or(1);
        sqlx::query!(
            "INSERT INTO pcp.fatores_sazonais (mes, fator, atualizado_em) VALUES ($1, $2, $3) \
             ON CONFLICT (mes) DO UPDATE SET fator = EXCLUDED.fator, \
             atualizado_em = EXCLUDED.atualizado_em",
            mes,
            fator,
            calculado_em
                .and_hms_opt(0, 0, 0)
                .unwrap_or_default()
                .and_utc(),
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

/// Vendas de um produto num mês (entrada do fator sazonal por produto — doc 02 §4).
#[derive(Debug, Clone)]
pub struct VendasProdutoMes {
    pub codigo_estoque: String,
    pub mes: i32,
    pub total: f64,
    pub dias: i64,
}

/// Vendas agregadas por (produto, mês) na janela — só dias COM venda (doc 02 §3.1).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn vendas_por_produto_mes(
    pool: &PgPool,
    inicio: NaiveDate,
    fim: NaiveDate,
) -> Result<Vec<VendasProdutoMes>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT codigo_estoque                  AS "codigo_estoque!",
                  EXTRACT(MONTH FROM dt_ref)::int4 AS "mes!",
                  SUM(qtd_vendida)::float8        AS "total!",
                  COUNT(DISTINCT dt_ref)          AS "dias!"
           FROM pcp.vendas_dia
           WHERE dt_ref >= $1 AND dt_ref < $2 AND qtd_vendida > 0
           GROUP BY 1, 2
           ORDER BY 1, 2"#,
        inicio,
        fim,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|l| VendasProdutoMes {
            codigo_estoque: l.codigo_estoque,
            mes: l.mes,
            total: l.total,
            dias: l.dias,
        })
        .collect())
}

/// Substitui TODOS os fatores por produto (full refresh, numa transação).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn substituir_por_produto(
    pool: &PgPool,
    fatores: &[(String, [f64; 12])],
) -> Result<u64, ErroDb> {
    let mut tx = pool.begin().await?;
    sqlx::query("TRUNCATE pcp.fator_sazonal_produto")
        .execute(&mut *tx)
        .await?;
    let mut gravadas = 0_u64;
    // Achata (produto, mês, fator) e grava em lotes (§15: nada de N+1).
    let linhas: Vec<(&str, i16, f64)> = fatores
        .iter()
        .flat_map(|(codigo, meses)| {
            meses
                .iter()
                .enumerate()
                .map(move |(i, &f)| (codigo.as_str(), i16::try_from(i + 1).unwrap_or(1), f))
        })
        .collect();
    for lote in linhas.chunks(5_000) {
        let mut qb = sqlx::QueryBuilder::new(
            "INSERT INTO pcp.fator_sazonal_produto (codigo_estoque, mes, fator) ",
        );
        qb.push_values(lote, |mut b, (codigo, mes, fator)| {
            b.push_bind(*codigo).push_bind(*mes).push_bind(*fator);
        });
        gravadas += qb.build().execute(&mut *tx).await?.rows_affected();
    }
    tx.commit().await?;
    Ok(gravadas)
}

/// Carrega os 12 fatores de cada produto que tem curva própria.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn carregar_por_produto(
    pool: &PgPool,
) -> Result<std::collections::HashMap<String, [f64; 12]>, ErroDb> {
    let linhas = sqlx::query!(
        "SELECT codigo_estoque, mes, fator FROM pcp.fator_sazonal_produto ORDER BY codigo_estoque"
    )
    .fetch_all(pool)
    .await?;
    let mut mapa: std::collections::HashMap<String, [f64; 12]> = std::collections::HashMap::new();
    for l in linhas {
        let entrada = mapa.entry(l.codigo_estoque).or_insert([1.0; 12]);
        if let Ok(i) = usize::try_from(l.mes - 1) {
            if i < 12 {
                entrada[i] = l.fator;
            }
        }
    }
    Ok(mapa)
}

/// Existe alguma curva sazonal por produto? (`false` força o primeiro cálculo — doc 02 §4.2.)
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn tem_curvas_por_produto(pool: &PgPool) -> Result<bool, ErroDb> {
    let existe = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM pcp.fator_sazonal_produto) AS "existe!""#
    )
    .fetch_one(pool)
    .await?;
    Ok(existe)
}
