//! Criação do pool de conexões Postgres, compartilhada por todos os módulos.

use sqlx::postgres::{PgPool, PgPoolOptions};

use crate::erro::ErroDb;

/// Cria um pool de conexões Postgres a partir da URL informada.
///
/// A URL deve vir de configuração/ambiente (ex.: `DATABASE_URL`), nunca hardcoded
/// (CLAUDE.md §7.4).
///
/// `search_path` é fixado em toda conexão. Convenção do `SuperFlow`: `public` vem
/// **primeiro**, para que o controle de migrations do módulo PCP (`_sqlx_migrations`, não
/// qualificado) viva sempre em `public` — independente do nome do role. As tabelas são
/// sempre referenciadas com schema explícito (`pcp.*`, `nucleo.*`, `catalogo.*` — §0).
///
/// # Errors
/// [`ErroDb::Sqlx`] se a conexão inicial ao banco falhar.
pub async fn criar_pool(
    database_url: &str,
    max_conexoes: u32,
    search_path: &str,
) -> Result<PgPool, ErroDb> {
    let search_path = search_path.to_owned();
    let pool = PgPoolOptions::new()
        .max_connections(max_conexoes)
        .after_connect(move |conn, _meta| {
            let search_path = search_path.clone();
            Box::pin(async move {
                // `SET search_path TO ...` não aceita parâmetro; `set_config` aceita. Assim o
                // SQL é estático e o valor vai como bind — nada de SQL dinâmico (CLAUDE.md §7).
                sqlx::query("SELECT set_config('search_path', $1, false)")
                    .bind(search_path)
                    .execute(&mut *conn)
                    .await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await?;
    Ok(pool)
}
