//! Casca (shell) HTML, `App` raiz e roteamento do frontend. Sem regra de negócio (CLAUDE.md §3).

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::StaticSegment;

/// Documento HTML servido no SSR (inclui scripts de hidratação e auto-reload em dev).
pub fn shell(options: LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1" />
                <link rel="icon" href="/favicon.svg" />
                <link rel="manifest" href="/manifest.webmanifest" />
                <meta name="theme-color" content="#FF6600" />
                <AutoReload options=options.clone() />
                <HydrationScripts options />
                <MetaTags />
            </head>
            <body>
                <App />
            </body>
        </html>
    }
}

/// Componente raiz: tema, metadados e roteador (CLAUDE.md §16).
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    view! {
        <Stylesheet id="leptos" href="/pkg/pcp-web.css" />
        <Title text="SuperCopo PCP" />
        <Router>
            <Routes fallback=|| view! { <p class="estado-vazio">"Página não encontrada."</p> }>
                <Route path=StaticSegment("") view=PaginaLogin />
            </Routes>
        </Router>
    }
}

/// Tela de login (consumirá a auth da API no próximo passo). Frontend burro (§3).
#[component]
fn PaginaLogin() -> impl IntoView {
    view! {
        <main class="login">
            <section class="login__cartao card">
                <img class="login__logo" src="/images/logo.svg" alt="SuperCopo PCP" />
                <h1 class="login__titulo">"Entrar no PCP"</h1>
                <p class="login__sub">"Planejamento e Controle de Produção"</p>
                <form class="login__form">
                    <label class="campo">
                        <span class="campo__rotulo">"E-mail"</span>
                        <input class="input" type="email" name="email" autocomplete="username" />
                    </label>
                    <label class="campo">
                        <span class="campo__rotulo">"Senha"</span>
                        <input
                            class="input"
                            type="password"
                            name="senha"
                            autocomplete="current-password"
                        />
                    </label>
                    <button class="btn btn--primario" type="submit">
                        "Entrar"
                    </button>
                </form>
            </section>
        </main>
    }
}
