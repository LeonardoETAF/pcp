//! Montagem do roteador Axum (CLAUDE.md §2/§7): rotas públicas de auth + subgrupo `/pcp`
//! protegido por middleware deny-by-default.

use axum::routing::{delete, get, post};
use axum::Router;

use crate::autenticacao::exigir_autenticacao;
use crate::estado::AppState;
use crate::leitura;
use crate::{filtros_salvos, handlers_auth, handlers_pcp, solicitacoes};

/// Constrói o roteador completo da API.
pub fn rotas(estado: AppState) -> Router {
    let protegidas = Router::new()
        .route("/pcp/me", get(handlers_pcp::me))
        .route("/pcp/aprovacoes", get(handlers_pcp::area_aprovacoes))
        .route("/pcp/usuarios", post(handlers_pcp::criar_usuario))
        .route("/pcp/dashboard", get(leitura::dashboard::dashboard))
        .route(
            "/pcp/dashboard/classes",
            get(leitura::dashboard_classes::classes),
        )
        .route("/pcp/estoque", get(leitura::estoque::estoque))
        .route(
            "/pcp/estoque/exportar",
            get(leitura::estoque_exportacao::exportar),
        )
        .route(
            "/pcp/estoque/filtros",
            get(filtros_salvos::listar).post(filtros_salvos::criar),
        )
        .route("/pcp/estoque/filtros/{id}", delete(filtros_salvos::excluir))
        .route("/pcp/produto/{codigo}", get(leitura::produto::produto))
        .route(
            "/pcp/solicitacoes",
            get(solicitacoes::listar).post(solicitacoes::criar),
        )
        .route(
            "/pcp/solicitacoes/{id}/transicao",
            post(solicitacoes::transicionar_estado),
        )
        .route("/pcp/alertas", get(leitura::alertas::alertas))
        .route("/pcp/abc", get(leitura::abc::abc))
        .route("/pcp/eventos", get(leitura::eventos::eventos))
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
