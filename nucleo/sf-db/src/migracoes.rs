//! Migrations do **núcleo** (`nucleo/migrations/`) — schema `nucleo`.
//!
//! Cada módulo do `SuperFlow` é dono das suas migrations (CLAUDE.md §0/§6). Para que os
//! controles não colidam, cada migrator grava a sua própria tabela `_sqlx_migrations`:
//! o núcleo grava em `nucleo` (fixando o `search_path` da conexão que roda o migrator) e
//! o módulo PCP segue gravando em `public`.

use sqlx::{Acquire, PgPool};

use crate::erro::ErroDb;

/// Aplica, em ordem, as migrations do núcleo. Idempotente.
///
/// # Errors
/// [`ErroDb::Sqlx`] se não for possível preparar a conexão;
/// [`ErroDb::Migracao`] se alguma migration falhar ou divergir do checksum registrado.
pub async fn aplicar_migrations_nucleo(pool: &PgPool) -> Result<(), ErroDb> {
    let mut conexao = pool.acquire().await?;

    // O schema precisa existir ANTES de o migrator criar o seu `_sqlx_migrations` nele.
    sqlx::query("CREATE SCHEMA IF NOT EXISTS nucleo")
        .execute(&mut *conexao)
        .await?;
    // `nucleo` primeiro: o `_sqlx_migrations` (não qualificado) do migrator nasce nele,
    // separado do controle do módulo PCP (que vive em `public`).
    sqlx::query("SET search_path TO nucleo, public")
        .execute(&mut *conexao)
        .await?;

    sqlx::migrate!("../migrations")
        .run(conexao.acquire().await?)
        .await?;
    Ok(())
}
