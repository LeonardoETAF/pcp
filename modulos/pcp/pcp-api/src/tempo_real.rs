//! Ponte de tempo real (CLAUDE.md §16): escuta o canal Postgres do pipeline (LISTEN/NOTIFY) e
//! repassa cada notificação para o canal broadcast da API, de onde o endpoint SSE serve os
//! clientes. Reconecta sozinha se a conexão cair (resiliência — sempre com fallback no cliente).

use std::time::Duration;

use tokio::sync::broadcast;

use pcp_db::eventos::criar_listener;

/// Intervalo entre tentativas de reconexão do listener.
const ESPERA_RECONEXAO: Duration = Duration::from_secs(3);

/// Laço infinito: mantém um `PgListener` vivo e publica os payloads no `emissor`.
/// Pensado para rodar em uma task dedicada (`tokio::spawn`).
pub async fn escutar_pipeline(url: String, emissor: broadcast::Sender<String>) {
    loop {
        match criar_listener(&url).await {
            Ok(mut listener) => loop {
                match listener.recv().await {
                    Ok(notificacao) => {
                        // Ignora o erro de "sem assinantes": é normal não haver SSE aberto.
                        let _ = emissor.send(notificacao.payload().to_owned());
                    }
                    Err(e) => {
                        tracing::warn!(erro = %e, "listener do pipeline caiu; reconectando");
                        break;
                    }
                }
            },
            Err(e) => {
                tracing::warn!(erro = %e, "falha ao criar listener do pipeline; tentando de novo");
            }
        }
        tokio::time::sleep(ESPERA_RECONEXAO).await;
    }
}
