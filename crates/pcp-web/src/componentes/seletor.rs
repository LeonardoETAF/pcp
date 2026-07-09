//! Seletor (dropdown) do design system, em substituição ao `<select>` nativo.
//!
//! O `<select>` abre uma lista desenhada pelo NAVEGADOR: fonte, cores, raio e espaçamento
//! escapam do design system e mudam de sistema operacional para sistema operacional. Aqui a lista
//! é markup nosso, estilizada pelos mesmos tokens dos cards (CLAUDE.md §16), e nada depende do
//! navegador — nem a lista, nem o fechamento por clique fora (um fundo transparente captura o
//! clique, sem ouvinte global de `window`).

use leptos::prelude::*;

use super::icone::Icone;

/// Uma opção da lista: `(valor, texto exibido)`.
pub type Opcao = (&'static str, &'static str);

#[component]
pub fn Seletor(
    /// Ícone de `public/icons` exibido à esquerda do valor atual.
    #[prop(optional)]
    icone: Option<&'static str>,
    /// Rótulo acessível do controle (vira `aria-label`).
    rotulo: &'static str,
    opcoes: Vec<Opcao>,
    /// Valor selecionado. Uma opção com este valor é a exibida no botão.
    valor: Signal<String>,
    /// Chamado com o valor da opção escolhida. Genérico em vez de `Callback` para não alocar:
    /// a closure some na compilação. `Send + Sync` porque o SSR renderiza a view entre threads.
    ao_escolher: impl Fn(String) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let aberto = RwSignal::new(false);
    let opcoes = StoredValue::new(opcoes);

    // Texto da opção atual. Sem correspondência (ex.: valor vazio), mostra a primeira opção.
    let texto_atual = move || {
        let v = valor.get();
        opcoes.with_value(|o| {
            o.iter()
                .find(|(val, _)| *val == v)
                .or_else(|| o.first())
                .map_or("", |(_, txt)| *txt)
        })
    };

    view! {
        <div
            class="seletor"
            class:seletor--aberto=move || aberto.get()
            on:keydown=move |ev| {
                if ev.key() == "Escape" {
                    aberto.set(false);
                }
            }
        >
            <button
                type="button"
                class="seletor__botao"
                aria-haspopup="listbox"
                aria-expanded=move || if aberto.get() { "true" } else { "false" }
                aria-label=rotulo
                on:click=move |_| aberto.update(|a| *a = !*a)
            >
                {icone.map(|arquivo| view! { <Icone arquivo /> })}
                <span class="seletor__valor">{texto_atual}</span>
                <span class="seletor__seta" aria-hidden="true">
                    <Icone arquivo="seta-baixo.svg" />
                </span>
            </button>

            <Show when=move || aberto.get() fallback=|| ()>
                // Fundo transparente: captura o clique fora e fecha a lista, sem ouvinte global.
                <button
                    type="button"
                    class="seletor__fundo"
                    tabindex="-1"
                    aria-hidden="true"
                    on:click=move |_| aberto.set(false)
                ></button>
                <ul class="seletor__lista" role="listbox" aria-label=rotulo>
                    {opcoes
                        .get_value()
                        .into_iter()
                        .map(|(val, txt)| {
                            let ativa = move || valor.get() == val;
                            view! {
                                <li role="option" aria-selected=move || ativa().to_string()>
                                    <button
                                        type="button"
                                        class="seletor__opcao"
                                        class:seletor__opcao--ativa=ativa
                                        on:click=move |_| {
                                            ao_escolher(val.to_owned());
                                            aberto.set(false);
                                        }
                                    >
                                        {txt}
                                    </button>
                                </li>
                            }
                        })
                        .collect_view()}
                </ul>
            </Show>
        </div>
    }
}
