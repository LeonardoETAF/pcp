//! Canal de eventos do pipeline via Postgres LISTEN/NOTIFY (CLAUDE.md §16). Desacopla o motor
//! (processo batch) do servidor HTTP: o motor NOTIFICA ao terminar; a API ESCUTA e repassa aos
//! clientes por SSE. Sem isso, a API não saberia, entre processos, que os dados mudaram.

use chrono::NaiveDate;
use sqlx::postgres::PgListener;
use sqlx::PgPool;

use crate::erro::ErroDb;

/// Canal Postgres usado para sinalizar o fim do processamento diário.
pub const CANAL_PIPELINE: &str = "pcp_pipeline";

/// Notifica os assinantes de que o pipeline de `data_ref` terminou com `status`.
/// Payload JSON mínimo `{"data_ref":..,"status":..}` (doc 05 §1.2 / §16). `status` é um código
/// controlado pelo motor (sem aspas/caracteres a escapar).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de banco.
pub async fn notificar_pipeline(
    pool: &PgPool,
    data_ref: NaiveDate,
    status: &str,
) -> Result<(), ErroDb> {
    let payload = format!(r#"{{"data_ref":"{data_ref}","status":"{status}"}}"#);
    sqlx::query("SELECT pg_notify($1, $2)")
        .bind(CANAL_PIPELINE)
        .bind(payload)
        .execute(pool)
        .await?;
    Ok(())
}

/// Cria um listener conectado e inscrito no [`CANAL_PIPELINE`] (consumido pela API).
///
/// # Errors
/// [`ErroDb::Sqlx`] em falha de conexão ou inscrição.
pub async fn criar_listener(url: &str) -> Result<PgListener, ErroDb> {
    let mut listener = PgListener::connect(url).await?;
    listener.listen(CANAL_PIPELINE).await?;
    Ok(listener)
}
