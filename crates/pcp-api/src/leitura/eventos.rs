//! `GET /pcp/eventos` — fluxo SSE de eventos do pipeline em tempo real (CLAUDE.md §16).
//! O cliente (pcp-web) assina e invalida a seção afetada ao receber `pipeline`; deve cair para
//! polling leve se a conexão cair.

use std::convert::Infallible;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::estado::AppState;

/// Abre um fluxo SSE para o assinante (autenticado — qualquer papel).
#[allow(clippy::unused_async)] // handler assíncrono exigido pelo Axum
pub async fn eventos(State(estado): State<AppState>) -> impl IntoResponse {
    let fluxo = BroadcastStream::new(estado.assinar()).filter_map(|msg| {
        // Receptor atrasado (Lagged) é descartado silenciosamente; o cliente fará polling.
        msg.ok().map(|payload| {
            Ok::<Event, Infallible>(Event::default().event("pipeline").data(payload))
        })
    });
    Sse::new(fluxo).keep_alive(KeepAlive::default())
}
