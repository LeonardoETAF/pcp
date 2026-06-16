//! Páginas operacionais ainda em construção (placeholders). Cada uma vira um arquivo/feature
//! própria nos prompts seguintes (2.5 Detalhe do Produto, 3.x ABC/Config/Chat). Frontend burro.

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
