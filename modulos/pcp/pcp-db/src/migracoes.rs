//! Aplicação das migrations versionadas (CLAUDE.md §6: nunca alterar schema só no banco).

use sqlx::PgPool;

use crate::erro::ErroDb;

/// Aplica, em ordem, todas as migrations do **módulo PCP** (`modulos/pcp/migrations/`).
/// Cada módulo do `SuperFlow` é dono das suas migrations e do seu schema (CLAUDE.md §0/§6).
/// Idempotente: migrations já aplicadas são puladas.
///
/// # Errors
/// [`ErroDb::Migracao`] se alguma migration falhar ou divergir do checksum registrado.
pub async fn aplicar_migrations(pool: &PgPool) -> Result<(), ErroDb> {
    sqlx::migrate!("../migrations").run(pool).await?;
    Ok(())
}
