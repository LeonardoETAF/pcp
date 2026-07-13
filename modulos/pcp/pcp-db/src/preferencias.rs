//! Preferências de exibição por usuário (`pcp.preferencia_usuario`, doc 03 §8). Cada usuário lê
//! e grava só as suas. Upsert simples; sem auditoria (preferência pessoal, não dado de negócio).

use sqlx::PgPool;
use uuid::Uuid;

use crate::erro::ErroDb;

/// Preferências de exibição de um usuário.
#[derive(Debug, Clone)]
pub struct Preferencia {
    pub pagina_inicial: String,
    pub tamanho_pagina: i32,
}

impl Default for Preferencia {
    fn default() -> Self {
        Self {
            pagina_inicial: "dashboard".to_owned(),
            tamanho_pagina: 50,
        }
    }
}

/// Preferências do usuário; o default se ainda não houver registro.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn obter(pool: &PgPool, usuario_id: Uuid) -> Result<Preferencia, ErroDb> {
    let r = sqlx::query!(
        "SELECT pagina_inicial, tamanho_pagina FROM pcp.preferencia_usuario WHERE usuario_id = $1",
        usuario_id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(r.map_or_else(Preferencia::default, |r| Preferencia {
        pagina_inicial: r.pagina_inicial,
        tamanho_pagina: r.tamanho_pagina,
    }))
}

/// Salva (upsert) as preferências do usuário.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn salvar(
    pool: &PgPool,
    usuario_id: Uuid,
    pagina_inicial: &str,
    tamanho_pagina: i32,
) -> Result<(), ErroDb> {
    sqlx::query!(
        "INSERT INTO pcp.preferencia_usuario (usuario_id, pagina_inicial, tamanho_pagina) \
         VALUES ($1, $2, $3) \
         ON CONFLICT (usuario_id) DO UPDATE \
           SET pagina_inicial = $2, tamanho_pagina = $3, atualizado_em = now()",
        usuario_id,
        pagina_inicial,
        tamanho_pagina,
    )
    .execute(pool)
    .await?;
    Ok(())
}
