//! Montagem do roteador Axum (CLAUDE.md §2/§7): rotas públicas de auth + subgrupo `/pcp`
//! protegido por middleware deny-by-default.

use axum::routing::{get, post};
use axum::Router;

use crate::autenticacao::exigir_autenticacao;
use crate::estado::AppState;
use crate::{handlers_auth, handlers_pcp};

/// Constrói o roteador completo da API.
pub fn rotas(estado: AppState) -> Router {
    let protegidas = Router::new()
        .route("/pcp/me", get(handlers_pcp::me))
        .route("/pcp/aprovacoes", get(handlers_pcp::area_aprovacoes))
        .route("/pcp/usuarios", post(handlers_pcp::criar_usuario))
        .route_layer(axum::middleware::from_fn_with_state(
            estado.clone(),
            exigir_autenticacao,
        ));

    let publicas = Router::new()
        .route("/saude", get(saude))
        .route("/auth/login", post(handlers_auth::login))
        .route("/auth/refresh", post(handlers_auth::refresh))
        .route("/auth/logout", post(handlers_auth::logout));

    Router::new()
        .merge(publicas)
        .merge(protegidas)
        .with_state(estado)
}

/// `GET /saude` — verificação simples de disponibilidade (pública).
#[allow(clippy::unused_async)] // handler assíncrono exigido pelo Axum
async fn saude() -> &'static str {
    "ok"
}
