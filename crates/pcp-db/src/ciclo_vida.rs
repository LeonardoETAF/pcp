//! Leitura e transição das sugestões de ciclo de vida (`pcp.sugestao_ciclo_vida`). A geração é do
//! motor (`derivadas`); aqui ficam a fila aberta e a transição de estado com auditoria inline
//! (quem aplicou/quando/observação — CLAUDE.md §7.5). A validação da máquina é do `pcp-core`.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::erro::ErroDb;

/// Uma sugestão de ciclo de vida (doc 04 §3.4).
#[derive(Debug, Clone)]
pub struct SugestaoCicloVida {
    pub id: Uuid,
    pub codigo_estoque: String,
    pub acao_sugerida: String,
    pub pontuacao: i16,
    pub nivel_certeza: String,
    pub criterios: Vec<String>,
    pub estado: String,
    pub data_analise: NaiveDate,
    pub aplicado_por: Option<String>,
    pub data_aplicacao: Option<DateTime<Utc>>,
    pub observacoes: Option<String>,
}

/// Fila de sugestões abertas (geradas / em análise), maior pontuação primeiro.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn listar_abertas(pool: &PgPool) -> Result<Vec<SugestaoCicloVida>, ErroDb> {
    let linhas = sqlx::query_as!(
        SugestaoCicloVida,
        r#"SELECT id, codigo_estoque, acao_sugerida, pontuacao, nivel_certeza,
                  criterios, estado, data_analise, aplicado_por, data_aplicacao, observacoes
           FROM pcp.sugestao_ciclo_vida
           WHERE estado IN ('gerada', 'em_analise')
           ORDER BY pontuacao DESC, codigo_estoque"#,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas)
}

/// Estado atual de uma sugestão (para validar a transição no chamador).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn estado_atual(pool: &PgPool, id: Uuid) -> Result<Option<String>, ErroDb> {
    let r = sqlx::query_scalar!(
        "SELECT estado FROM pcp.sugestao_ciclo_vida WHERE id = $1",
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(r)
}

/// Aplica uma transição já validada, registrando quem agiu/quando (auditoria inline §7.5).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn transicionar(
    pool: &PgPool,
    id: Uuid,
    para: &str,
    aplicado_por: &str,
    observacao: Option<&str>,
) -> Result<SugestaoCicloVida, ErroDb> {
    let s = sqlx::query_as!(
        SugestaoCicloVida,
        r#"UPDATE pcp.sugestao_ciclo_vida
           SET estado = $2,
               aplicado_por = $3,
               data_aplicacao = now(),
               observacoes = COALESCE($4, observacoes),
               atualizado_em = now()
           WHERE id = $1
           RETURNING id, codigo_estoque, acao_sugerida, pontuacao, nivel_certeza,
                     criterios, estado, data_analise, aplicado_por, data_aplicacao, observacoes"#,
        id,
        para,
        aplicado_por,
        observacao,
    )
    .fetch_one(pool)
    .await?;
    Ok(s)
}
