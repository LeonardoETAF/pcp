//! Persistência da configuração de negócio efetiva (`pcp.config_efetiva`) e da auditoria de
//! mudanças (`pcp.config_auditoria`). O conteúdo é opaco aqui (jsonb) — quem valida/tipa é o
//! `pcp-api` via `pcp-config` (fronteira de módulo, §0). Escrita auditada (§7.5).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::erro::ErroDb;

/// Uma constante alterada (para a trilha de auditoria).
#[derive(Debug, Clone)]
pub struct MudancaConfig {
    pub chave: String,
    pub valor_anterior: Option<String>,
    pub valor_novo: Option<String>,
}

/// Uma entrada da auditoria de configuração.
#[derive(Debug, Clone)]
pub struct EntradaAuditoria {
    pub chave: String,
    pub valor_anterior: Option<String>,
    pub valor_novo: Option<String>,
    pub por_id: Uuid,
    pub em: DateTime<Utc>,
}

/// Carrega a config efetiva persistida (jsonb). `None` se ainda não houve edição (usa o YAML).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn carregar(pool: &PgPool) -> Result<Option<serde_json::Value>, ErroDb> {
    let r = sqlx::query_scalar!(r#"SELECT valor AS "valor!" FROM pcp.config_efetiva WHERE id"#)
        .fetch_optional(pool)
        .await?;
    Ok(r)
}

/// Salva a nova config efetiva e registra cada constante alterada, numa transação (§7.5).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn salvar(
    pool: &PgPool,
    valor: &serde_json::Value,
    por_id: Uuid,
    mudancas: &[MudancaConfig],
) -> Result<(), ErroDb> {
    let mut tx = pool.begin().await?;
    sqlx::query!(
        r#"INSERT INTO pcp.config_efetiva (id, valor, atualizado_em, atualizado_por)
           VALUES (true, $1, now(), $2)
           ON CONFLICT (id) DO UPDATE SET valor = $1, atualizado_em = now(), atualizado_por = $2"#,
        valor,
        por_id,
    )
    .execute(&mut *tx)
    .await?;
    for m in mudancas {
        sqlx::query!(
            r#"INSERT INTO pcp.config_auditoria (chave, valor_anterior, valor_novo, por_id)
               VALUES ($1, $2, $3, $4)"#,
            m.chave,
            m.valor_anterior,
            m.valor_novo,
            por_id,
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Últimas entradas da auditoria de configuração (mais recentes primeiro).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn auditoria(pool: &PgPool, limite: i64) -> Result<Vec<EntradaAuditoria>, ErroDb> {
    let linhas = sqlx::query_as!(
        EntradaAuditoria,
        r#"SELECT chave, valor_anterior, valor_novo, por_id, em
           FROM pcp.config_auditoria ORDER BY em DESC LIMIT $1"#,
        limite,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas)
}
