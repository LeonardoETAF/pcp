//! Binário do servidor SSR do frontend (Axum + `leptos_axum`). Serve a app e os assets de `public/`.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
// `view!` aninhadas geram tipos profundos; o release resolve o layout completo e estoura o
// limite padrão (128). Sobe o limite (não afeta runtime).
#![recursion_limit = "512"]

#[cfg(feature = "ssr")]
#[tokio::main]
async fn main() {
    use axum::Router;
    use leptos::logging::log;
    use leptos::prelude::*;
    use leptos_axum::{generate_route_list, LeptosRoutes};
    use pcp_web::app::{shell, App};

    let conf = get_configuration(None).expect("configuração do Leptos");
    let leptos_options = conf.leptos_options;
    let addr = leptos_options.site_addr;
    let routes = generate_route_list(App);

    let app = Router::new()
        // Endpoints das server functions (login etc.) — server-to-server com a pcp-api.
        .route(
            "/api/{*fn_name}",
            axum::routing::post(leptos_axum::handle_server_fns),
        )
        .leptos_routes(&leptos_options, routes, {
            let leptos_options = leptos_options.clone();
            move || shell(leptos_options.clone())
        })
        .fallback(leptos_axum::file_and_error_handler(shell))
        .with_state(leptos_options);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("bind do listener");
    log!("pcp-web (SSR) ouvindo em http://{addr}");
    axum::serve(listener, app.into_make_service())
        .await
        .expect("servidor axum");
}

// Sob a feature `hydrate` (build WASM) não há binário de servidor.
#[cfg(not(feature = "ssr"))]
fn main() {}
