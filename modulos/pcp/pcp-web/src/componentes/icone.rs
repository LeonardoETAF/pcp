//! Ícone do diretório `public/icons`, recolorido com a cor do texto via CSS mask. Nenhum SVG
//! embutido no código: o arquivo é sempre um asset do projeto.

use leptos::prelude::*;

/// Máscara CSS que aponta para um arquivo de `public/icons`.
pub fn mascara(arquivo: &str) -> String {
    format!("-webkit-mask-image:url(/icons/{arquivo});mask-image:url(/icons/{arquivo})")
}

#[component]
pub fn Icone(arquivo: &'static str) -> impl IntoView {
    view! { <span class="icone-mask" style=mascara(arquivo)></span> }
}
