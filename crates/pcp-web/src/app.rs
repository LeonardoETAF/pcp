//! Casca (shell) HTML, `App` raiz, sessão e roteamento. Sem regra de negócio (CLAUDE.md §3).

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::use_navigate;
use leptos_router::{NavigateOptions, StaticSegment};

use crate::api::Login;

/// Sessão do usuário no cliente: o `access_token` (em memória). Persistência/cookie httpOnly
/// fica como endurecimento de produção (frontend burro — só guarda o token da API).
#[derive(Clone, Copy)]
struct Sessao(RwSignal<Option<String>>);

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

/// Componente raiz: tema, metadados, sessão e roteador (CLAUDE.md §16).
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    provide_context(Sessao(RwSignal::new(None)));
    view! {
        <Stylesheet id="leptos" href="/pkg/pcp-web.css" />
        <Title text="SuperCopo PCP" />
        <Router>
            <Routes fallback=|| {
                view! { <p class="estado-vazio">"Página não encontrada."</p> }
            }>
                <Route path=StaticSegment("") view=PaginaLogin />
                <Route path=StaticSegment("app") view=PaginaApp />
            </Routes>
        </Router>
    }
}

/// Tela de login: dispara a server function `login` e, em sucesso, guarda o token e vai p/ `/app`.
#[component]
fn PaginaLogin() -> impl IntoView {
    let login = ServerAction::<Login>::new();
    let sessao = expect_context::<Sessao>();
    let navegar = use_navigate();

    Effect::new(move |_| {
        if let Some(Ok(token)) = login.value().get() {
            sessao.0.set(Some(token));
            navegar("/app", NavigateOptions::default());
        }
    });

    let mensagem_erro = move || match login.value().get() {
        Some(Err(e)) => Some(e.to_string()),
        _ => None,
    };

    view! {
        <main class="login">
            <section class="login__cartao card">
                <img class="login__logo" src="/images/logo.svg" alt="SuperCopo PCP" />
                <h1 class="login__titulo">"Entrar no PCP"</h1>
                <p class="login__sub">"Planejamento e Controle de Produção"</p>
                <ActionForm action=login attr:class="login__form">
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
                    {move || {
                        mensagem_erro()
                            .map(|e| view! { <p class="login__erro" role="alert">{e}</p> })
                    }}
                    <button
                        class="btn btn--primario"
                        type="submit"
                        prop:disabled=move || login.pending().get()
                    >
                        {move || if login.pending().get() { "Entrando…" } else { "Entrar" }}
                    </button>
                </ActionForm>
            </section>
        </main>
    }
}

/// Página autenticada (placeholder). Gate: sem token na sessão, volta para o login.
#[component]
fn PaginaApp() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let navegar = use_navigate();

    Effect::new(move |_| {
        if sessao.0.get().is_none() {
            navegar("/", NavigateOptions::default());
        }
    });

    let sair = move |_| sessao.0.set(None);

    view! {
        <Show
            when=move || sessao.0.get().is_some()
            fallback=|| view! { <p class="estado-vazio">"Redirecionando…"</p> }
        >
            <header class="appbar">
                <img class="appbar__logo" src="/images/logo.svg" alt="SuperCopo PCP" />
                <button class="btn btn--secundario" on:click=sair>
                    "Sair"
                </button>
            </header>
            <main class="conteudo">
                <h1>"Bem-vindo ao PCP"</h1>
                <p class="texto-suave">
                    "Autenticado. As telas operacionais entram nos próximos prompts."
                </p>
            </main>
        </Show>
    }
}
