//! Pool de conexões do módulo PCP.
//!
//! A criação do pool é infra do núcleo ([`sf_db::criar_pool`]); aqui só se fixa o
//! `search_path` do PCP: `public` primeiro (onde vive o `_sqlx_migrations` deste módulo),
//! depois `nucleo` (identidade) e `pcp` (tabelas do módulo).

use sqlx::postgres::PgPool;

use crate::erro::ErroDb;

/// `search_path` do módulo PCP. As tabelas são sempre referenciadas com schema explícito;
/// a ordem aqui só decide onde nascem objetos não qualificados (ex.: `_sqlx_migrations`).
const SEARCH_PATH: &str = "public, nucleo, pcp";

/// Cria o pool de conexões do PCP a partir da URL informada.
///
/// A URL deve vir de configuração/ambiente (ex.: `DATABASE_URL`), nunca hardcoded
/// (CLAUDE.md §7.4).
///
/// # Errors
/// [`ErroDb::Sqlx`] se a conexão inicial ao banco falhar.
pub async fn criar_pool(database_url: &str, max_conexoes: u32) -> Result<PgPool, ErroDb> {
    sf_db::criar_pool(database_url, max_conexoes, SEARCH_PATH).await
}
