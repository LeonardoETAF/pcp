//! Montagem do roteador Axum (CLAUDE.md §2/§7): rotas públicas de auth + subgrupo `/pcp`
//! protegido por middleware deny-by-default.

use axum::routing::{delete, get, post};
use axum::Router;

use crate::autenticacao::exigir_autenticacao;
use crate::estado::AppState;
use crate::leitura;
use crate::{
    ciclo_vida, config, filtros_salvos, handlers_auth, handlers_pcp, preferencias, sazonalidade,
    solicitacoes, usuarios,
};

/// Constrói o roteador completo da API.
pub fn rotas(estado: AppState) -> Router {
    let protegidas = Router::new()
        .route("/pcp/me", get(handlers_pcp::me))
        .route("/pcp/config", get(config::obter).put(config::salvar))
        .route("/pcp/config/auditoria", get(config::auditoria))
        .route("/pcp/aprovacoes", get(handlers_pcp::area_aprovacoes))
        .route(
            "/pcp/usuarios",
            get(usuarios::listar).post(handlers_pcp::criar_usuario),
        )
        .route(
            "/pcp/usuarios/{id}",
            axum::routing::put(usuarios::atualizar),
        )
        .route(
            "/pcp/preferencias",
            get(preferencias::obter).put(preferencias::salvar),
        )
        .route("/pcp/sazonalidade", get(sazonalidade::listar))
        .route("/pcp/sazonalidade/auditoria", get(sazonalidade::auditoria))
        .route(
            "/pcp/sazonalidade/{mes}",
            axum::routing::put(sazonalidade::override_mes),
        )
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
            "/pcp/produto/{codigo}/insights",
            get(leitura::insights::insights),
        )
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
        .route("/pcp/abc/tabela", get(leitura::abc_tabela::abc_tabela))
        .route("/pcp/ciclo-vida", get(ciclo_vida::fila))
        .route(
            "/pcp/ciclo-vida/{id}/transicao",
            post(ciclo_vida::transicionar_estado),
        )
        .route("/pcp/admin/pipeline", get(leitura::operacao::pipeline))
        .route("/pcp/admin/saude", get(leitura::operacao::saude))
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
