//! Layout autenticado: sidebar de navegação (mapa do doc 03) + header (tema/sair) + `<Outlet/>`.
//! Gate de autenticação: sem token na sessão, redireciona ao login. Frontend burro (§3/§16).

use leptos::prelude::*;
use leptos_router::components::{Outlet, A};
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;

use crate::contexto::{Sessao, Tema};

#[component]
pub fn LayoutAutenticado() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let tema = expect_context::<Tema>();
    // `StoredValue` é Copy: permite capturar o `navegar` em handlers dentro de `<Show>` (Fn).
    let navegar = StoredValue::new(use_navigate());

    // Gate: sem token -> login.
    Effect::new(move |_| {
        if sessao.0.get().is_none() {
            navegar.with_value(|n| n("/login", NavigateOptions::default()));
        }
    });

    let alternar_tema = move |_| {
        tema.0
            .update(|t| *t = if *t == "escuro" { "claro" } else { "escuro" });
    };
    let sair = move |_| {
        sessao.0.set(None);
        navegar.with_value(|n| n("/login", NavigateOptions::default()));
    };

    view! {
        <Show
            when=move || sessao.0.get().is_some()
            fallback=|| view! { <p class="estado-vazio">"Redirecionando…"</p> }
        >
            <div class="app-shell">
                <aside class="sidebar">
                    <img class="sidebar__logo" src="/images/logo.svg" alt="SuperCopo PCP" />
                    <nav class="nav">
                        <A href="/dashboard" attr:class="nav__link">
                            "Dashboard"
                        </A>
                        <A href="/estoque" attr:class="nav__link">
                            "Estoque"
                        </A>
                        <A href="/alertas" attr:class="nav__link">
                            "Alertas"
                        </A>
                        <A href="/abc" attr:class="nav__link">
                            "Classificação ABC"
                        </A>
                        <A href="/ai-chat" attr:class="nav__link">
                            "Chat IA"
                        </A>
                        <A href="/configuracoes" attr:class="nav__link">
                            "Configurações"
                        </A>
                    </nav>
                </aside>
                <div class="app-main">
                    <header class="topbar">
                        <span class="topbar__titulo">"PCP — SuperCopo"</span>
                        <div class="topbar__acoes">
                            <button
                                class="btn btn--secundario"
                                type="button"
                                on:click=alternar_tema
                            >
                                {move || {
                                    if tema.0.get() == "escuro" { "☀ Claro" } else { "🌙 Escuro" }
                                }}
                            </button>
                            <button class="btn btn--secundario" type="button" on:click=sair>
                                "Sair"
                            </button>
                        </div>
                    </header>
                    <main class="conteudo">
                        <Outlet />
                    </main>
                </div>
            </div>
        </Show>
    }
}
