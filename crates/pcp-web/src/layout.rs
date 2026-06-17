//! Shell autenticado (padrão do mockup): sidebar escura recolhível (logo centralizada clicável,
//! seção do módulo, item ativo em pílula, rodapé de usuário) e topbar (título da rota + tema +
//! notificações). Ícones do diretório `public/icons` recoloridos via CSS mask (herdam a cor).
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
        _ => "Dashboard",
    }
}

#[component]
pub fn LayoutAutenticado() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let tema = expect_context::<Tema>();
    let navegar = StoredValue::new(use_navigate());
    let local = use_location();
    let recolhido = RwSignal::new(false);

    Effect::new(move |_| {
        if sessao.0.get().is_none() {
            navegar.with_value(|n| n("/login", NavigateOptions::default()));
        }
    });

    let alternar_tema = move |_| {
        tema.0
            .update(|t| *t = if *t == "escuro" { "claro" } else { "escuro" });
    };
    let titulo = move || titulo_da_rota(&local.pathname.get());

    view! {
        <Show
            when=move || sessao.0.get().is_some()
            fallback=|| view! { <p class="estado-vazio">"Redirecionando…"</p> }
        >
            <div class="shell" class:shell--recolhido=move || recolhido.get()>
                <BarraLateral recolhido />
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
                                <span
                                    class="icone-mask"
                                    style=move || {
                                        let arq = if tema.0.get() == "escuro" {
                                            "modo-claro.svg"
                                        } else {
                                            "modo-escuro.svg"
                                        };
                                        format!(
                                            "-webkit-mask-image:url(/icons/{arq});mask-image:url(/icons/{arq})",
                                        )
                                    }
                                ></span>
                            </button>
                            <A href="/alertas" attr:class="icone-btn" attr:aria-label="Alertas">
                                <Icone arquivo="sino-notificacao.svg" />
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
    // Type-erasing (CLAUDE.md §16): colapsa o tipo profundo desta view para `AnyView`. Sem isso,
    // o SSR em release compõe o tipo do layout com o de cada rota-filha e estoura a resolução de
    // tipos do rustc ("queries overflow the depth limit"). Não tem custo de runtime relevante.
    .into_any()
}

/// Sidebar escura: logo clicável (recolhe/expande), módulos do PCP e rodapé de usuário.
#[component]
fn BarraLateral(recolhido: RwSignal<bool>) -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let navegar = StoredValue::new(use_navigate());
    let papel = Resource::new(
        move || sessao.0.get(),
        |token| async move {
            match token {
                Some(t) => perfil(t).await.unwrap_or_default(),
                None => String::new(),
            }
        },
    );
    let sair = move |_| {
        sessao.0.set(None);
        navegar.with_value(|n| n("/login", NavigateOptions::default()));
    };
    let logo_src = move || {
        if recolhido.get() {
            "/images/simbolo-branco.svg"
        } else {
            "/images/logo-branco.svg"
        }
    };

    view! {
        <aside class="barra">
            <button
                class="barra__marca"
                type="button"
                aria-label="Recolher ou expandir o menu"
                on:click=move |_| recolhido.update(|r| *r = !*r)
            >
                <img class="barra__logo" src=logo_src alt="SuperCopo" />
            </button>
            <p class="barra__secao">"PCP"</p>
            <div class="barra__divisor"></div>
            <nav class="menu">
                <ItemMenu href="/dashboard" rotulo="Dashboard" icone="layout-sidebar.svg" />
                <ItemMenu href="/estoque" rotulo="Estoque" icone="estoque-inventario.svg" />
                <ItemMenu href="/alertas" rotulo="Alertas" icone="sino-notificacao.svg" />
                <ItemMenu href="/abc" rotulo="Classificação ABC" icone="relatorios-kpis.svg" />
                <ItemMenu href="/configuracoes" rotulo="Configurações" icone="engrenagem.svg" />
            </nav>
            <div class="usuario">
                <span class="usuario__avatar">
                    <Icone arquivo="usuario.svg" />
                </span>
                <div class="usuario__dados">
                    <span class="usuario__nome">"Minha conta"</span>
                    <span class="usuario__papel">{move || papel.get().unwrap_or_default()}</span>
                </div>
                <button
                    class="usuario__sair"
                    type="button"
                    on:click=sair
                    aria-label="Sair"
                    title="Sair"
                >
                    <Icone arquivo="sair.svg" />
                </button>
            </div>
        </aside>
    }
    // Type-erasing: a sidebar (menus + rodapé) é a subárvore mais profunda; colapsá-la para
    // `AnyView` mantém raso o tipo do layout e evita o estouro de resolução do SSR em release.
    .into_any()
}

/// Ícone do diretório `public/icons` recolorido com a cor do texto (CSS mask).
#[component]
fn Icone(arquivo: &'static str) -> impl IntoView {
    let estilo =
        format!("-webkit-mask-image:url(/icons/{arquivo});mask-image:url(/icons/{arquivo})");
    view! { <span class="icone-mask" style=estilo></span> }
}

#[component]
fn ItemMenu(href: &'static str, rotulo: &'static str, icone: &'static str) -> impl IntoView {
    view! {
        <A href=href attr:class="menu__item" attr:title=rotulo>
            <Icone arquivo=icone />
            <span class="menu__rotulo">{rotulo}</span>
        </A>
    }
}
