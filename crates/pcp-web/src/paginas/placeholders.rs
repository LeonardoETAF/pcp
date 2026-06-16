//! Páginas operacionais (placeholders). Cada uma vira um arquivo/feature própria nos prompts
//! seguintes (2.3 Alertas, 2.4 Estoque, 2.5 Produto, 3.x Dashboard/ABC/Config). Frontend burro.

use leptos::prelude::*;

#[component]
fn EmConstrucao(titulo: &'static str) -> impl IntoView {
    view! {
        <section class="placeholder">
            <h1 class="placeholder__titulo">{titulo}</h1>
            <p class="texto-suave">"Em construção — entra nos próximos prompts."</p>
        </section>
    }
}

#[component]
pub fn Dashboard() -> impl IntoView {
    view! { <EmConstrucao titulo="Dashboard" /> }
}

#[component]
pub fn Estoque() -> impl IntoView {
    view! { <EmConstrucao titulo="Gestão de Estoque" /> }
}

#[component]
pub fn DetalheProduto() -> impl IntoView {
    view! { <EmConstrucao titulo="Detalhe do Produto" /> }
}

#[component]
pub fn ClassificacaoAbc() -> impl IntoView {
    view! { <EmConstrucao titulo="Classificação ABC" /> }
}

#[component]
pub fn ChatIa() -> impl IntoView {
    view! { <EmConstrucao titulo="Chat IA" /> }
}

#[component]
pub fn Configuracoes() -> impl IntoView {
    view! { <EmConstrucao titulo="Configurações" /> }
}
