//! Repositório de refresh tokens revogáveis (`pcp.refresh_token`). Guarda apenas o hash.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::erro::ErroDb;

/// Refresh token persistido (somente o hash do valor real).
#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub id: Uuid,
    pub usuario_id: Uuid,
    pub token_hash: String,
    pub expira_em: DateTime<Utc>,
    pub revogado: bool,
    pub criado_em: DateTime<Utc>,
}

/// Persiste um novo refresh token (hash + validade).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn criar(
    pool: &PgPool,
    usuario_id: Uuid,
    token_hash: &str,
    expira_em: DateTime<Utc>,
) -> Result<(), ErroDb> {
    sqlx::query!(
        "INSERT INTO pcp.refresh_token (usuario_id, token_hash, expira_em) VALUES ($1, $2, $3)",
        usuario_id,
        token_hash,
        expira_em,
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Busca um refresh token VÁLIDO (existe, não revogado, não expirado) pelo hash.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn buscar_valido(
    pool: &PgPool,
    token_hash: &str,
) -> Result<Option<RefreshToken>, ErroDb> {
    let token = sqlx::query_as!(
        RefreshToken,
        "SELECT id, usuario_id, token_hash, expira_em, revogado, criado_em \
         FROM pcp.refresh_token \
         WHERE token_hash = $1 AND revogado = false AND expira_em > now()",
        token_hash,
    )
    .fetch_optional(pool)
    .await?;
    Ok(token)
}

/// Revoga um refresh token pelo hash (logout). Idempotente.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn revogar(pool: &PgPool, token_hash: &str) -> Result<(), ErroDb> {
    sqlx::query!(
        "UPDATE pcp.refresh_token SET revogado = true WHERE token_hash = $1",
        token_hash,
    )
    .execute(pool)
    .await?;
    Ok(())
}
