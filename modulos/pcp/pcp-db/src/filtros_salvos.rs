//! Repositório de filtros salvos da Gestão de Estoque (`pcp.filtro_salvo`). Persistência pura,
//! escopada ao usuário dono. O conteúdo do filtro é opaco aqui (jsonb → [`serde_json::Value`]);
//! quem o interpreta é o `pcp-web` (fronteira de módulo — CLAUDE.md §0).

use sqlx::PgPool;
use uuid::Uuid;

use crate::erro::ErroDb;

/// Um filtro salvo do usuário (preferência de UI).
#[derive(Debug, Clone)]
pub struct FiltroSalvo {
    pub id: Uuid,
    pub nome: String,
    pub filtro: serde_json::Value,
}

/// Lista os filtros do usuário, em ordem alfabética.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn listar(pool: &PgPool, usuario_id: Uuid) -> Result<Vec<FiltroSalvo>, ErroDb> {
    let linhas = sqlx::query!(
        r#"SELECT id, nome, filtro FROM pcp.filtro_salvo
           WHERE usuario_id = $1 ORDER BY nome"#,
        usuario_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas
        .into_iter()
        .map(|r| FiltroSalvo {
            id: r.id,
            nome: r.nome,
            filtro: r.filtro,
        })
        .collect())
}

/// Salva (ou atualiza, por nome) um filtro do usuário. `nome` já vem aparado e não-vazio.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn salvar(
    pool: &PgPool,
    usuario_id: Uuid,
    nome: &str,
    filtro: &serde_json::Value,
) -> Result<FiltroSalvo, ErroDb> {
    let r = sqlx::query!(
        r#"INSERT INTO pcp.filtro_salvo (usuario_id, nome, filtro)
           VALUES ($1, $2, $3)
           ON CONFLICT (usuario_id, nome)
             DO UPDATE SET filtro = EXCLUDED.filtro, criado_em = now()
           RETURNING id, nome, filtro"#,
        usuario_id,
        nome,
        filtro,
    )
    .fetch_one(pool)
    .await?;
    Ok(FiltroSalvo {
        id: r.id,
        nome: r.nome,
        filtro: r.filtro,
    })
}

/// Exclui um filtro do usuário (só remove se for dele). Retorna `true` se algo foi removido.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn excluir(pool: &PgPool, usuario_id: Uuid, id: Uuid) -> Result<bool, ErroDb> {
    let r = sqlx::query!(
        "DELETE FROM pcp.filtro_salvo WHERE id = $1 AND usuario_id = $2",
        id,
        usuario_id,
    )
    .execute(pool)
    .await?;
    Ok(r.rows_affected() > 0)
}
