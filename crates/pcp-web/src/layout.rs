//! Shell autenticado no padrão do mockup: sidebar escura ("MÓDULOS" + item ativo em pílula +
//! rodapé de usuário) e topbar (título da rota + tema + notificações). Gate de autenticação.
//! Só os módulos do PCP (CLAUDE.md §0). Frontend burro (§3).

use leptos::prelude::*;
use leptos_router::components::{Outlet, A};
use leptos_router::hooks::{use_location, use_navigate};
use leptos_router::NavigateOptions;

use crate::api::perfil;
use crate::contexto::{Sessao, Tema};

fn titulo_da_rota(path: &str) -> &'static str {
    match path {
        "/estoque" => "Gestão de Estoque",
        "/alertas" => "Central de Alertas",
        "/abc" => "Classificação ABC",
        "/ai-chat" => "Chat IA",
        "/configuracoes" => "Configurações",
        p if p.starts_with("/estoque/") => "Detalhe do Produto",
        _ => "Visão geral",
    }
}

#[component]
pub fn LayoutAutenticado() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let tema = expect_context::<Tema>();
    let navegar = StoredValue::new(use_navigate());
    let local = use_location();

    Effect::new(move |_| {
        if sessao.0.get().is_none() {
            navegar.with_value(|n| n("/login", NavigateOptions::default()));
        }
    });

    let papel = Resource::new(
        move || sessao.0.get(),
        |token| async move {
            match token {
                Some(t) => perfil(t).await.unwrap_or_default(),
                None => String::new(),
            }
        },
    );

    let alternar_tema = move |_| {
        tema.0
            .update(|t| *t = if *t == "escuro" { "claro" } else { "escuro" });
    };
    let sair = move |_| {
        sessao.0.set(None);
        navegar.with_value(|n| n("/login", NavigateOptions::default()));
    };
    let titulo = move || titulo_da_rota(&local.pathname.get());

    view! {
        <Show
            when=move || sessao.0.get().is_some()
            fallback=|| view! { <p class="estado-vazio">"Redirecionando…"</p> }
        >
            <div class="shell">
                <aside class="barra">
                    <img class="barra__logo" src="/images/logo-branco.svg" alt="SuperCopo" />
                    <p class="barra__secao">"Módulos"</p>
                    <nav class="menu">
                        <ItemMenu href="/dashboard" rotulo="Visão geral">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="3" width="7.5" height="7.5" rx="1.6"/><rect x="13.5" y="3" width="7.5" height="7.5" rx="1.6"/><rect x="3" y="13.5" width="7.5" height="7.5" rx="1.6"/><rect x="13.5" y="13.5" width="7.5" height="7.5" rx="1.6"/></svg>
                        </ItemMenu>
                        <ItemMenu href="/estoque" rotulo="Estoque">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M21 8 12 3 3 8l9 5 9-5Z"/><path d="M3 8v8l9 5 9-5V8"/></svg>
                        </ItemMenu>
                        <ItemMenu href="/alertas" rotulo="Alertas">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M18 8a6 6 0 1 0-12 0c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.7 21a2 2 0 0 1-3.4 0"/></svg>
                        </ItemMenu>
                        <ItemMenu href="/abc" rotulo="Classificação ABC">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M4 20V10M10 20V4M16 20v-7M22 20H2"/></svg>
                        </ItemMenu>
                        <ItemMenu href="/ai-chat" rotulo="Chat IA">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H8l-5 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2Z"/></svg>
                        </ItemMenu>
                        <ItemMenu href="/configuracoes" rotulo="Configurações">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.6 1.6 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.6 1.6 0 0 0-2.7 1.1V21a2 2 0 0 1-4 0v-.1A1.6 1.6 0 0 0 7 19.4a1.6 1.6 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.6 1.6 0 0 0-1.1-2.7H1a2 2 0 0 1 0-4h.1A1.6 1.6 0 0 0 2.6 7a1.6 1.6 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.6 1.6 0 0 0 1.8.3H7a1.6 1.6 0 0 0 1-1.5V1a2 2 0 0 1 4 0v.1a1.6 1.6 0 0 0 2.7 1.1 1.6 1.6 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.6 1.6 0 0 0-.3 1.8V7a1.6 1.6 0 0 0 1.5 1H23a2 2 0 0 1 0 4h-.1a1.6 1.6 0 0 0-1.5 1Z"/></svg>
                        </ItemMenu>
                    </nav>
                    <div class="usuario">
                        <span class="usuario__avatar">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="8" r="4"/><path d="M5 21a7 7 0 0 1 14 0"/></svg>
                        </span>
                        <div class="usuario__dados">
                            <span class="usuario__nome">"Minha conta"</span>
                            <span class="usuario__papel">
                                {move || papel.get().unwrap_or_default()}
                            </span>
                        </div>
                        <button class="usuario__sair" type="button" on:click=sair aria-label="Sair">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4"/><path d="m16 17 5-5-5-5M21 12H9"/></svg>
                        </button>
                    </div>
                </aside>
                <div class="conteudo-area">
                    <header class="topbar">
                        <h1 class="topbar__titulo">{titulo}</h1>
                        <div class="topbar__acoes">
                            <button
                                class="icone-btn"
                                type="button"
                                aria-label="Alternar tema"
                                on:click=alternar_tema
                            >
                                {move || if tema.0.get() == "escuro" { "☀" } else { "🌙" }}
                            </button>
                            <A href="/alertas" attr:class="icone-btn" attr:aria-label="Alertas">
                                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M18 8a6 6 0 1 0-12 0c0 7-3 9-3 9h18s-3-2-3-9"/><path d="M13.7 21a2 2 0 0 1-3.4 0"/></svg>
                            </A>
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

#[component]
fn ItemMenu(href: &'static str, rotulo: &'static str, children: Children) -> impl IntoView {
    view! {
        <A href=href attr:class="menu__item">
            <span class="menu__icone">{children()}</span>
            <span class="menu__rotulo">{rotulo}</span>
        </A>
    }
}
