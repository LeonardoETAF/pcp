//! Expurgo de retenção (CLAUDE.md §9/§13): apaga dados que excedem a janela definida em
//! `pcp.retencao_politica` — a fonte ÚNICA das regras de retenção. Sem isto, tabelas acumulam
//! indefinidamente (foi o pior débito do legado). Idempotente: rodar de novo não tem efeito extra.
//!
//! O SQL é montado em runtime a partir das linhas da política (dataset/coluna/condição controlados
//! por migration — confiáveis, não entrada de usuário); a janela em dias vai como bind param.

use sqlx::{PgPool, Row};

use crate::erro::ErroDb;

/// Aplica todas as políticas de retenção com janela definida e limpa os refresh tokens expirados.
/// Retorna `(dataset, linhas_removidas)` por alvo, para telemetria/log.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn expurgar(pool: &PgPool) -> Result<Vec<(String, u64)>, ErroDb> {
    let politicas = sqlx::query(
        "SELECT dataset, retencao_dias, base_coluna, condicao_extra \
         FROM pcp.retencao_politica WHERE retencao_dias IS NOT NULL ORDER BY dataset",
    )
    .fetch_all(pool)
    .await?;

    let mut resultado = Vec::with_capacity(politicas.len() + 1);
    for linha in &politicas {
        let dataset: String = linha.get("dataset");
        let dias: i32 = linha.get("retencao_dias");
        let base: String = linha.get("base_coluna");
        let condicao: Option<String> = linha.try_get("condicao_extra").ok().flatten();
        let extra = condicao.map_or_else(String::new, |c| format!(" AND ({c})"));
        // Identificadores vêm da política (controlada por migration, NÃO entrada de usuário) e a
        // janela em dias vai por bind — auditado seguro contra injeção (`AssertSqlSafe`, sqlx 0.9).
        let sql =
            format!("DELETE FROM {dataset} WHERE {base} < now() - ($1 * interval '1 day'){extra}");
        let removidas = sqlx::query(sqlx::AssertSqlSafe(sql))
            .bind(dias)
            .execute(pool)
            .await?
            .rows_affected();
        resultado.push((dataset, removidas));
    }

    // Higiene de segurança (§7): refresh tokens vencidos (retenção por expiração, não por idade).
    let tokens = sqlx::query("DELETE FROM pcp.refresh_token WHERE expira_em < now()")
        .execute(pool)
        .await?
        .rows_affected();
    resultado.push(("pcp.refresh_token".to_owned(), tokens));

    Ok(resultado)
}
