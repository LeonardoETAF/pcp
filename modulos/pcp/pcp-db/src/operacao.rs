//! Leituras de operação (doc 05 §3/§4): status das execuções do pipeline e métricas para os
//! health checks. Só LÊ tabelas derivadas/log; a avaliação de limiares (ok/atenção/crítico) é
//! feita na `pcp-api` (frontend burro — §3). Sem regra de negócio aqui.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;

use crate::erro::ErroDb;

/// Uma execução de módulo do pipeline (linha de `pcp.execucao_pipeline`).
#[derive(Debug, Clone)]
pub struct RegistroExecucao {
    pub data_ref: NaiveDate,
    pub modulo: String,
    pub status: String,
    pub linhas_afetadas: i64,
    pub duracao_ms: i64,
    pub erro: Option<String>,
    pub inicio: DateTime<Utc>,
    pub fim: DateTime<Utc>,
}

/// Execuções mais recentes do pipeline (por início desc), para o painel de operação.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn execucoes_recentes(
    pool: &PgPool,
    limite: i64,
) -> Result<Vec<RegistroExecucao>, ErroDb> {
    let linhas = sqlx::query_as!(
        RegistroExecucao,
        "SELECT data_ref, modulo, status, linhas_afetadas, duracao_ms, erro, inicio, fim \
         FROM pcp.execucao_pipeline ORDER BY inicio DESC LIMIT $1",
        limite,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas)
}

/// Métricas cruas para os health checks (doc 05 §4). Os limiares são aplicados na API.
#[derive(Debug, Clone, Default)]
pub struct MetricasSaude {
    /// Data do snapshot mais recente em `estoque_snapshot` (None se vazio).
    pub data_ref_snapshot: Option<NaiveDate>,
    /// Nº de produtos no snapshot mais recente.
    pub produtos_recente: i64,
    /// Nº de produtos no snapshot do dia imediatamente anterior (para variação).
    pub produtos_anterior: i64,
    /// Duração total (soma dos módulos) da última execução do pipeline, em ms.
    pub duracao_ultima_ms: i64,
    /// Duração média total por execução (sobre todas as `data_ref`), em ms.
    pub duracao_media_ms: f64,
    /// Houve erro em algum módulo da última execução?
    pub ultima_execucao_com_erro: bool,
    /// Dias desde o alerta mais recente (None se nunca houve alerta).
    pub dias_sem_alerta: Option<i64>,
    /// Coeficiente de variação médio do catálogo ativo (exclui sentinela 999 de sem histórico).
    pub cv_medio: Option<f64>,
}

/// Coleta as métricas dos health checks. `hoje` entra como parâmetro (regra pura de tempo — §5).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn metricas_saude(pool: &PgPool, hoje: NaiveDate) -> Result<MetricasSaude, ErroDb> {
    let data_ref_snapshot = sqlx::query_scalar!("SELECT MAX(dt_ref) FROM pcp.estoque_snapshot")
        .fetch_one(pool)
        .await?;

    let (produtos_recente, produtos_anterior) = match data_ref_snapshot {
        Some(recente) => {
            let r = sqlx::query_scalar!(
                r#"SELECT COUNT(*) AS "n!" FROM pcp.estoque_snapshot WHERE dt_ref = $1"#,
                recente,
            )
            .fetch_one(pool)
            .await?;
            let anterior_data = sqlx::query_scalar!(
                "SELECT MAX(dt_ref) FROM pcp.estoque_snapshot WHERE dt_ref < $1",
                recente,
            )
            .fetch_one(pool)
            .await?;
            let a =
                match anterior_data {
                    Some(d) => sqlx::query_scalar!(
                        r#"SELECT COUNT(*) AS "n!" FROM pcp.estoque_snapshot WHERE dt_ref = $1"#,
                        d,
                    )
                    .fetch_one(pool)
                    .await?,
                    None => 0,
                };
            (r, a)
        }
        None => (0, 0),
    };

    // Duração total por execução (soma dos módulos por data_ref): última vs média das execuções.
    let duracao_ultima_ms = sqlx::query_scalar!(
        r#"SELECT COALESCE(SUM(duracao_ms), 0)::bigint AS "ms!" FROM pcp.execucao_pipeline
           WHERE data_ref = (SELECT MAX(data_ref) FROM pcp.execucao_pipeline)"#,
    )
    .fetch_one(pool)
    .await?;
    let duracao_media_ms = sqlx::query_scalar!(
        r#"SELECT COALESCE(AVG(total), 0)::float8 AS "media!" FROM
           (SELECT SUM(duracao_ms) AS total FROM pcp.execucao_pipeline GROUP BY data_ref) t"#,
    )
    .fetch_one(pool)
    .await?;
    let ultima_execucao_com_erro = sqlx::query_scalar!(
        r#"SELECT EXISTS(SELECT 1 FROM pcp.execucao_pipeline
           WHERE data_ref = (SELECT MAX(data_ref) FROM pcp.execucao_pipeline) AND status = 'erro') AS "e!""#,
    )
    .fetch_one(pool)
    .await?;

    let dias_sem_alerta = sqlx::query_scalar!(
        r#"SELECT ($1::date - MAX(dt_alerta)) AS "dias" FROM pcp.alerta"#,
        hoje,
    )
    .fetch_one(pool)
    .await?
    .map(i64::from);

    let cv_medio = sqlx::query_scalar!(
        r#"SELECT AVG(coef_variacao)::float8 AS "cv" FROM pcp.produto_ativo WHERE cobertura_dias < 999"#,
    )
    .fetch_one(pool)
    .await?;

    Ok(MetricasSaude {
        data_ref_snapshot,
        produtos_recente,
        produtos_anterior,
        duracao_ultima_ms,
        duracao_media_ms,
        ultima_execucao_com_erro,
        dias_sem_alerta,
        cv_medio,
    })
}
