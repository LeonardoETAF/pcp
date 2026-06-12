//! Criação do pool de conexões Postgres.

use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::erro::ErroDb;

/// Cria um pool de conexões Postgres a partir da URL informada.
///
/// A URL deve vir de configuração/ambiente (ex.: `DATABASE_URL`), nunca hardcoded
/// (CLAUDE.md §7.4).
///
/// Fixa o `search_path` em `public, pcp` para que o controle de migrations
/// (`_sqlx_migrations`, não qualificado) viva sempre em `public` — independente do nome do
/// role. As tabelas do módulo são sempre referenciadas como `pcp.*` (CLAUDE.md §0).
///
/// # Errors
/// [`ErroDb::Sqlx`] se a conexão inicial ao banco falhar.
pub async fn criar_pool(database_url: &str, max_conexoes: u32) -> Result<PgPool, ErroDb> {
    let pool = PgPoolOptions::new()
        .max_connections(max_conexoes)
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO public, pcp")
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await?;
    Ok(pool)
}
