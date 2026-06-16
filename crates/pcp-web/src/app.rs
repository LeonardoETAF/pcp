//! Casca (shell) HTML, `App` raiz, contextos e roteamento. Só wiring — componentes ficam em
//! `layout.rs` e `paginas/` (um por arquivo, §15). Sem regra de negócio (CLAUDE.md §3).

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::{ParamSegment, StaticSegment};

use crate::contexto::{Sessao, Tema};
use crate::layout::LayoutAutenticado;
use crate::paginas::login::PaginaLogin;
use crate::paginas::placeholders::{
    Alertas, ChatIa, ClassificacaoAbc, Configuracoes, Dashboard, DetalheProduto, Estoque,
};

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

/// Componente raiz: tema, metadados, contextos (sessão/tema) e roteador (CLAUDE.md §16).
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();
    let sessao = Sessao(RwSignal::new(None));
    let tema = Tema(RwSignal::new("claro"));
    provide_context(sessao);
    provide_context(tema);

    view! {
        <Stylesheet id="leptos" href="/pkg/pcp-web.css" />
        <Title text="SuperCopo PCP" />
        <div class="app-raiz" data-tema=move || tema.0.get()>
            <Router>
                <Routes fallback=|| {
                    view! { <p class="estado-vazio">"Página não encontrada."</p> }
                }>
                    <Route path=StaticSegment("login") view=PaginaLogin />
                    <ParentRoute path=StaticSegment("") view=LayoutAutenticado>
                        <Route path=StaticSegment("") view=Dashboard />
                        <Route path=StaticSegment("dashboard") view=Dashboard />
                        <Route path=StaticSegment("estoque") view=Estoque />
                        <Route
                            path=(StaticSegment("estoque"), ParamSegment("codigo"))
                            view=DetalheProduto
                        />
                        <Route path=StaticSegment("alertas") view=Alertas />
                        <Route path=StaticSegment("abc") view=ClassificacaoAbc />
                        <Route path=StaticSegment("ai-chat") view=ChatIa />
                        <Route path=StaticSegment("configuracoes") view=Configuracoes />
                    </ParentRoute>
                </Routes>
            </Router>
        </div>
    }
}
