//! Migrations do **núcleo** (`nucleo/migrations/`) — schema `nucleo`.
//!
//! Cada módulo do `SuperFlow` é dono das suas migrations (CLAUDE.md §0/§6). Para que os
//! controles não colidam, cada migrator grava a sua própria tabela `_sqlx_migrations`:
//! o núcleo grava em `nucleo` (fixando o `search_path` da conexão que roda o migrator) e
//! o módulo PCP segue gravando em `public`.

use sqlx::{Acquire, PgPool};

use crate::erro::ErroDb;

/// Chave do advisory lock que serializa a criação do schema `nucleo`. Valor arbitrário, só
/// precisa ser estável e não colidir com outro lock do sistema.
const LOCK_CRIACAO_SCHEMA: i64 = 0x5355_5045_5246_4C57; // "SUPERFLW" em ASCII

/// Aplica, em ordem, as migrations do núcleo. Idempotente e seguro sob concorrência.
///
/// # Errors
/// [`ErroDb::Sqlx`] se não for possível preparar a conexão;
/// [`ErroDb::Migracao`] se alguma migration falhar ou divergir do checksum registrado.
pub async fn aplicar_migrations_nucleo(pool: &PgPool) -> Result<(), ErroDb> {
    let mut conexao = pool.acquire().await?;

    // O schema precisa existir ANTES de o migrator criar o seu `_sqlx_migrations` nele.
    //
    // `CREATE SCHEMA IF NOT EXISTS` NÃO é seguro sob concorrência no Postgres: dois processos
    // podem passar pelo `IF NOT EXISTS` ao mesmo tempo e um leva erro de chave duplicada em
    // `pg_namespace`. Isso acontece de verdade — a `pcp-api` e o `sync_one` sobem juntos no
    // compose. O advisory lock (liberado no fim da transação) serializa a criação.
    let mut tx = conexao.begin().await?;
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(LOCK_CRIACAO_SCHEMA)
        .execute(&mut *tx)
        .await?;
    sqlx::query("CREATE SCHEMA IF NOT EXISTS nucleo")
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    // `nucleo` primeiro: o `_sqlx_migrations` (não qualificado) do migrator nasce nele,
    // separado do controle do módulo PCP (que vive em `public`).
    sqlx::query("SET search_path TO nucleo, public")
        .execute(&mut *conexao)
        .await?;

    // O migrator do sqlx já toma o seu próprio lock — daqui em diante a concorrência é dele.
    sqlx::migrate!("../migrations")
        .run(conexao.acquire().await?)
        .await?;
    Ok(())
}
