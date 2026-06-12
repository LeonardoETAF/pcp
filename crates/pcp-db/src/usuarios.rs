//! Repositório de usuários (`pcp.usuario`). Persistência pura — a regra de papéis/auth
//! vive no `pcp-api` (CLAUDE.md §7). O `papel` é guardado como texto (CHECK no banco).

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::erro::ErroDb;

/// Usuário do sistema. `senha_hash` é o hash argon2 — nunca expor em resposta de API.
#[derive(Debug, Clone)]
pub struct Usuario {
    pub id: Uuid,
    pub email: String,
    pub senha_hash: String,
    pub papel: String,
    pub nome: Option<String>,
    pub ativo: bool,
    pub criado_em: DateTime<Utc>,
}

/// Cria um usuário. `email` deve vir normalizado (minúsculo) e `papel` validado pelo chamador.
///
/// # Errors
/// [`ErroDb::Sqlx`] (ex.: violação de unicidade do e-mail ou do CHECK de papel).
pub async fn criar(
    pool: &PgPool,
    email: &str,
    senha_hash: &str,
    papel: &str,
    nome: Option<&str>,
) -> Result<Usuario, ErroDb> {
    let usuario = sqlx::query_as!(
        Usuario,
        "INSERT INTO pcp.usuario (email, senha_hash, papel, nome) VALUES ($1, $2, $3, $4) \
         RETURNING id, email, senha_hash, papel, nome, ativo, criado_em",
        email,
        senha_hash,
        papel,
        nome,
    )
    .fetch_one(pool)
    .await?;
    Ok(usuario)
}

/// Busca um usuário pelo e-mail (já normalizado).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn buscar_por_email(pool: &PgPool, email: &str) -> Result<Option<Usuario>, ErroDb> {
    let usuario = sqlx::query_as!(
        Usuario,
        "SELECT id, email, senha_hash, papel, nome, ativo, criado_em \
         FROM pcp.usuario WHERE email = $1",
        email,
    )
    .fetch_optional(pool)
    .await?;
    Ok(usuario)
}

/// Busca um usuário pelo identificador.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn buscar_por_id(pool: &PgPool, id: Uuid) -> Result<Option<Usuario>, ErroDb> {
    let usuario = sqlx::query_as!(
        Usuario,
        "SELECT id, email, senha_hash, papel, nome, ativo, criado_em \
         FROM pcp.usuario WHERE id = $1",
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(usuario)
}

/// Conta os usuários cadastrados (apoio ao bootstrap do admin inicial).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn contar(pool: &PgPool) -> Result<i64, ErroDb> {
    let total = sqlx::query_scalar!("SELECT COUNT(*) FROM pcp.usuario")
        .fetch_one(pool)
        .await?;
    Ok(total.unwrap_or(0))
}
