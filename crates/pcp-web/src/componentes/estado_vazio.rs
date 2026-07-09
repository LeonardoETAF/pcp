//! Estado vazio ilustrado (CLAUDE.md §16: "estados vazios/erro/loading cuidados").
//!
//! As ilustrações vêm de `public/empty-states`. São decorativas: o texto ao lado já diz tudo, então
//! a imagem sai da árvore de acessibilidade (`alt=""` + `aria-hidden`).
//!
//! Usado onde o vazio ocupa a área principal. Dentro de cards pequenos (gráficos, mini-listas) a
//! ilustração não cabe — ali segue valendo o `<p class="estado-vazio">` só com texto.

use leptos::prelude::*;

#[component]
pub fn EstadoVazio(
    /// Arquivo de `public/empty-states` (ex.: `"empty-search.svg"`).
    arte: &'static str,
    titulo: &'static str,
    /// Linha de apoio: o que aconteceu, ou o que fazer a seguir.
    #[prop(optional)]
    descricao: Option<&'static str>,
) -> impl IntoView {
    view! {
        <div class="estado-vazio estado-vazio--ilustrado">
            <img
                class="estado-vazio__arte"
                src=format!("/empty-states/{arte}")
                alt=""
                aria-hidden="true"
            />
            <p class="estado-vazio__titulo">{titulo}</p>
            {descricao.map(|d| view! { <p class="estado-vazio__texto">{d}</p> })}
        </div>
    }
}
