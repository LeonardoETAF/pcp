//! Tela de login split-screen (marca à esquerda + painel à direita). O painel alterna entre três
//! vistas: entrar, recuperar senha e falar com o administrador. Frontend burro (CLAUDE.md §3).

mod marca;
mod vistas;

use leptos::prelude::*;

use marca::PainelMarca;
use vistas::{VistaContato, VistaLogin, VistaRecuperar};

/// Vista ativa do painel direito.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Vista {
    Login,
    Recuperar,
    Contato,
}

/// Ícone do diretório `public/icons`, recolorido pela cor do texto (CSS mask). Nenhum SVG é
/// embutido no código: todo ícone da tela de login sai de um arquivo do projeto.
#[component]
pub fn Icone(arquivo: &'static str) -> impl IntoView {
    let estilo =
        format!("-webkit-mask-image:url(/icons/{arquivo});mask-image:url(/icons/{arquivo})");
    view! { <span class="icone-mask" style=estilo></span> }
}

#[component]
pub fn PaginaLogin() -> impl IntoView {
    let vista = RwSignal::new(Vista::Login);
    view! {
        <div class="auth">
            <PainelMarca />
            <div class="auth__painel">
                <div class="painel__caixa">
                    {move || match vista.get() {
                        Vista::Login => view! { <VistaLogin vista /> }.into_any(),
                        Vista::Recuperar => view! { <VistaRecuperar vista /> }.into_any(),
                        Vista::Contato => view! { <VistaContato vista /> }.into_any(),
                    }}
                </div>
            </div>
        </div>
    }
}
