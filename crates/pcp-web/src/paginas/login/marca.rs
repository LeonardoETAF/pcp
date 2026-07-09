//! Painel esquerdo (marca): logo, headline, features e rodapé. Estático. Os ícones vêm SEMPRE do
//! diretório `public/icons` (nenhum SVG embutido no código) e são recoloridos via CSS mask, então
//! herdam `currentColor` (= branco sobre o fundo escuro).

use leptos::prelude::*;

use super::Icone;

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
                            <Icone arquivo="apontamento.svg" />
                        </span>
                        <span>"Pedidos de produção e apontamento"</span>
                    </li>
                    <li class="feature">
                        <span class="feature__icone">
                            <Icone arquivo="lista.svg" />
                        </span>
                        <span>"Planejamento e cronograma"</span>
                    </li>
                    <li class="feature">
                        <span class="feature__icone">
                            <Icone arquivo="inventory.svg" />
                        </span>
                        <span>"Estoque e inventário em tempo real"</span>
                    </li>
                </ul>
            </div>
            <p class="marca__rodape">"© 2026 Supercopo. Todos os direitos reservados."</p>
        </aside>
    }
}
