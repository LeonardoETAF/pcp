//! Botões de paginação do design system: setas + até 5 páginas numeradas (janela deslizante).
//! Compartilhado pela lista de estoque e pelos históricos do detalhe do produto.

use leptos::prelude::*;

use super::icone::Icone;
use crate::formato::fmt_milhar;

/// Quantas páginas numeradas cabem entre as setas.
const JANELA_PAGINAS: i64 = 5;

/// Janela de até [`JANELA_PAGINAS`] páginas em volta da atual, sem estourar as bordas: perto do
/// início ou do fim ela não encolhe, desliza — o usuário sempre vê o mesmo número de botões.
fn janela(atual: i64, total_paginas: i64) -> impl Iterator<Item = i64> {
    let largura = JANELA_PAGINAS.min(total_paginas);
    let primeira = (atual - largura / 2)
        .max(1)
        .min((total_paginas - largura + 1).max(1));
    primeira..primeira + largura
}

/// Setas e páginas numeradas. `total` é a contagem de itens; a página tem `limite` itens.
#[component]
pub fn PaginacaoBotoes(
    limite: RwSignal<i64>,
    deslocamento: RwSignal<i64>,
    total: i64,
) -> impl IntoView {
    let tem_anterior = move || deslocamento.get() > 0;
    let tem_proximo = move || deslocamento.get() + limite.get() < total;
    let total_paginas = move || (total + limite.get() - 1) / limite.get().max(1);
    let atual = move || deslocamento.get() / limite.get().max(1) + 1;
    let ir_para = move |pagina: i64| deslocamento.set((pagina - 1) * limite.get());
    view! {
        <div class="paginacao__botoes">
            <button
                type="button"
                class="paginacao__seta"
                aria-label="Página anterior"
                disabled=move || (!tem_anterior()).then_some("")
                on:click=move |_| {
                    deslocamento.update(|d| *d = (*d - limite.get()).max(0));
                }
            >
                <Icone arquivo="seta-esquerda.svg" />
            </button>
            {move || {
                janela(atual(), total_paginas())
                    .map(|pagina| {
                        let ativa = pagina == atual();
                        view! {
                            <button
                                type="button"
                                class="paginacao__pagina"
                                class:paginacao__pagina--ativa=ativa
                                aria-current=ativa.then_some("page")
                                on:click=move |_| ir_para(pagina)
                            >
                                {fmt_milhar(pagina)}
                            </button>
                        }
                    })
                    .collect_view()
            }}
            <button
                type="button"
                class="paginacao__seta"
                aria-label="Próxima página"
                disabled=move || (!tem_proximo()).then_some("")
                on:click=move |_| {
                    deslocamento.update(|d| *d += limite.get());
                }
            >
                <Icone arquivo="seta-direita.svg" />
            </button>
        </div>
    }
}
