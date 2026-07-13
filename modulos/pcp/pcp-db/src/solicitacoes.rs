//! Repositório de Solicitações de Produção (`pcp.solicitacao_producao` + `pcp.solicitacao_evento`).
//! Escrita do usuário, auditada (CLAUDE.md §7.5): cada criação/transição registra um evento com
//! quem/quando/valor anterior. A validação da máquina de estados é do `pcp-core` (chamador).

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::erro::ErroDb;

/// Uma solicitação de produção persistida.
#[derive(Debug, Clone)]
pub struct Solicitacao {
    pub id: Uuid,
    pub codigo_estoque: String,
    pub qtd_solicitada: i64,
    pub prioridade: String,
    pub lead_time_dias: i32,
    pub prazo: NaiveDate,
    pub solicitante_id: Uuid,
    pub justificativa: Option<String>,
    pub estado: String,
    pub criado_em: DateTime<Utc>,
    pub atualizado_em: DateTime<Utc>,
}

/// Dados de criação de uma solicitação (a quantidade/prioridade vêm da recomendação, editáveis).
#[derive(Debug, Clone)]
pub struct NovaSolicitacao<'a> {
    pub codigo_estoque: &'a str,
    pub qtd_solicitada: i64,
    pub prioridade: &'a str,
    pub lead_time_dias: i32,
    pub prazo: NaiveDate,
    pub solicitante_id: Uuid,
    pub justificativa: Option<&'a str>,
    /// Estado inicial: `pendente` ou `aprovada` (aprovação automática — doc 02 §7.2).
    pub estado_inicial: &'a str,
}

/// Um evento de auditoria da solicitação (§7.5).
#[derive(Debug, Clone)]
pub struct EventoSolicitacao {
    pub de_estado: Option<String>,
    pub para_estado: String,
    pub por_id: Uuid,
    pub observacao: Option<String>,
    pub em: DateTime<Utc>,
}

/// Cria a solicitação e registra o evento inicial (`de_estado` NULL) numa transação.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn criar(pool: &PgPool, nova: &NovaSolicitacao<'_>) -> Result<Solicitacao, ErroDb> {
    let mut tx = pool.begin().await?;
    let s = sqlx::query_as!(
        Solicitacao,
        r#"INSERT INTO pcp.solicitacao_producao
             (codigo_estoque, qtd_solicitada, prioridade, lead_time_dias, prazo,
              solicitante_id, justificativa, estado)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, codigo_estoque, qtd_solicitada, prioridade, lead_time_dias, prazo,
                     solicitante_id, justificativa, estado, criado_em, atualizado_em"#,
        nova.codigo_estoque,
        nova.qtd_solicitada,
        nova.prioridade,
        nova.lead_time_dias,
        nova.prazo,
        nova.solicitante_id,
        nova.justificativa,
        nova.estado_inicial,
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO pcp.solicitacao_evento (solicitacao_id, de_estado, para_estado, por_id, observacao)
           VALUES ($1, NULL, $2, $3, $4)"#,
        s.id,
        s.estado,
        nova.solicitante_id,
        nova.justificativa,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(s)
}

/// Solicitações de um produto, mais recentes primeiro.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn listar_por_produto(pool: &PgPool, codigo: &str) -> Result<Vec<Solicitacao>, ErroDb> {
    let linhas = sqlx::query_as!(
        Solicitacao,
        r#"SELECT id, codigo_estoque, qtd_solicitada, prioridade, lead_time_dias, prazo,
                  solicitante_id, justificativa, estado, criado_em, atualizado_em
           FROM pcp.solicitacao_producao
           WHERE codigo_estoque = $1 ORDER BY criado_em DESC"#,
        codigo,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas)
}

/// Estado atual de uma solicitação (para validar a transição no chamador).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn estado_atual(pool: &PgPool, id: Uuid) -> Result<Option<String>, ErroDb> {
    let r = sqlx::query_scalar!(
        "SELECT estado FROM pcp.solicitacao_producao WHERE id = $1",
        id,
    )
    .fetch_optional(pool)
    .await?;
    Ok(r)
}

/// Aplica uma transição já validada (`de` → `para`) e registra o evento, numa transação.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn transicionar(
    pool: &PgPool,
    id: Uuid,
    de: &str,
    para: &str,
    por_id: Uuid,
    observacao: Option<&str>,
) -> Result<Solicitacao, ErroDb> {
    let mut tx = pool.begin().await?;
    let s = sqlx::query_as!(
        Solicitacao,
        r#"UPDATE pcp.solicitacao_producao
           SET estado = $2, atualizado_em = now()
           WHERE id = $1
           RETURNING id, codigo_estoque, qtd_solicitada, prioridade, lead_time_dias, prazo,
                     solicitante_id, justificativa, estado, criado_em, atualizado_em"#,
        id,
        para,
    )
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query!(
        r#"INSERT INTO pcp.solicitacao_evento (solicitacao_id, de_estado, para_estado, por_id, observacao)
           VALUES ($1, $2, $3, $4, $5)"#,
        id,
        de,
        para,
        por_id,
        observacao,
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(s)
}

/// Histórico de eventos (auditoria) de uma solicitação, em ordem cronológica.
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn eventos(
    pool: &PgPool,
    solicitacao_id: Uuid,
) -> Result<Vec<EventoSolicitacao>, ErroDb> {
    let linhas = sqlx::query_as!(
        EventoSolicitacao,
        r#"SELECT de_estado, para_estado AS "para_estado!", por_id, observacao, em
           FROM pcp.solicitacao_evento
           WHERE solicitacao_id = $1 ORDER BY em"#,
        solicitacao_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(linhas)
}
