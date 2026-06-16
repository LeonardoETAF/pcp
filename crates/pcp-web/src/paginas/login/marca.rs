//! Painel esquerdo (marca): logo, headline, features e rodapé. Estático. Ícones inline (herdam
//! `currentColor` = branco sobre o fundo escuro).

use leptos::prelude::*;

#[component]
pub fn PainelMarca() -> impl IntoView {
    view! {
        <aside class="auth__marca">
            <img class="marca__watermark" src="/images/simbolo-branco.svg" alt="" aria-hidden="true" />
            <div class="marca__topo">
                <img class="marca__logo" src="/images/logo-branco.svg" alt="SuperCopo" />
                <p class="marca__sub">"Planejamento e Controle de Produção"</p>
            </div>
            <div class="marca__centro">
                <h2 class="marca__titulo">
                    <span class="marca__titulo-linha">"Do chão de fábrica ao pedido"</span>
                    <span class="marca__titulo-linha">"tudo em um só sistema."</span>
                </h2>
                <p class="marca__texto">
                    "Acompanhe pedidos, planejamento, estoque e expedição em tempo real, com indicadores que ajudam a decidir."
                </p>
                <ul class="marca__features">
                    <li class="feature">
                        <span class="feature__icone">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                                <rect x="6" y="4" width="12" height="17" rx="2" />
                                <path d="M9 4h6v3H9z" />
                                <path d="m9 13 2 2 4-4" />
                            </svg>
                        </span>
                        <span>"Pedidos de produção e apontamento"</span>
                    </li>
                    <li class="feature">
                        <span class="feature__icone">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M4 6h9" />
                                <path d="M4 12h6" />
                                <path d="M4 18h13" />
                                <path d="M13 6v0M10 12v0M17 18v0" />
                            </svg>
                        </span>
                        <span>"Planejamento e cronograma"</span>
                    </li>
                    <li class="feature">
                        <span class="feature__icone">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                                <path d="M21 8 12 3 3 8l9 5 9-5Z" />
                                <path d="M3 8v8l9 5 9-5V8" />
                            </svg>
                        </span>
                        <span>"Estoque e inventário em tempo real"</span>
                    </li>
                </ul>
            </div>
            <p class="marca__rodape">"© 2026 Supercopo. Todos os direitos reservados."</p>
        </aside>
    }
}
