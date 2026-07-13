//! Casca (shell) HTML, `App` raiz, contextos e roteamento. Só wiring — componentes ficam em
//! `layout.rs` e `paginas/` (um por arquivo, §15). Sem regra de negócio (CLAUDE.md §3).

use leptos::prelude::*;
use leptos_meta::{provide_meta_context, MetaTags, Stylesheet, Title};
use leptos_router::components::{ParentRoute, Route, Router, Routes};
use leptos_router::{ParamSegment, StaticSegment};

use crate::componentes::EstadoVazio;
use crate::contexto::{CarregandoSessao, FiltroEstoque, Sessao, Tema};
use crate::layout::LayoutAutenticado;
use crate::paginas::abc::ClassificacaoAbc;
use crate::paginas::alertas::PaginaAlertas;
use crate::paginas::configuracoes::Configuracoes;
use crate::paginas::dashboard::PaginaDashboard;
use crate::paginas::estoque::PaginaEstoque;
use crate::paginas::login::PaginaLogin;
use crate::paginas::operacao::Operacao;
use crate::paginas::placeholders::ChatIa;
use crate::paginas::produto::DetalheProdutoPagina;

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
    // Já nasce "carregando" se há refresh salvo (leitura síncrona; no-op no SSR), para o layout
    // não redirecionar ao login antes de a restauração tentar renovar a sessão.
    let tem_refresh = crate::armazenamento::ler(crate::armazenamento::REFRESH).is_some();
    let carregando = CarregandoSessao(RwSignal::new(tem_refresh));
    provide_context(sessao);
    provide_context(tema);
    provide_context(carregando);
    // A lista de estoque guarda busca/filtros/página aqui, para sobreviverem à ida ao detalhe
    // de um produto e à volta (§16).
    provide_context(FiltroEstoque::default());

    // Restaura a sessão após reload: se há refresh token salvo, renova o access token. Roda só no
    // cliente (no SSR `tem_refresh` é falso). Falha = refresh inválido/expirado → cai no login.
    Effect::new(move |_| {
        if !tem_refresh || sessao.0.get_untracked().is_some() {
            return;
        }
        if let Some(refresh) = crate::armazenamento::ler(crate::armazenamento::REFRESH) {
            leptos::task::spawn_local(async move {
                match crate::api::renovar_sessao(refresh).await {
                    Ok(token) => sessao.0.set(Some(token)),
                    Err(_) => crate::armazenamento::remover(crate::armazenamento::REFRESH),
                }
                carregando.0.set(false);
            });
        } else {
            carregando.0.set(false);
        }
    });

    view! {
        <Stylesheet id="leptos" href="/pkg/pcp-web.css" />
        <Title text="SuperCopo PCP" />
        <div class="app-raiz" data-tema=move || tema.0.get()>
            <Router>
                <Routes fallback=|| {
                    view! {
                        <EstadoVazio
                            arte="empty-search.svg"
                            titulo="Página não encontrada"
                            descricao="O endereço não existe ou foi movido."
                        />
                    }
                }>
                    <Route path=StaticSegment("login") view=PaginaLogin />
                    <ParentRoute path=StaticSegment("") view=LayoutAutenticado>
                        <Route path=StaticSegment("") view=PaginaDashboard />
                        <Route path=StaticSegment("dashboard") view=PaginaDashboard />
                        <Route path=StaticSegment("estoque") view=PaginaEstoque />
                        <Route
                            path=(StaticSegment("estoque"), ParamSegment("codigo"))
                            view=DetalheProdutoPagina
                        />
                        <Route path=StaticSegment("alertas") view=PaginaAlertas />
                        <Route path=StaticSegment("abc") view=ClassificacaoAbc />
                        <Route path=StaticSegment("ai-chat") view=ChatIa />
                        <Route path=StaticSegment("configuracoes") view=Configuracoes />
                        <Route path=StaticSegment("operacao") view=Operacao />
                    </ParentRoute>
                </Routes>
            </Router>
        </div>
    }
}
