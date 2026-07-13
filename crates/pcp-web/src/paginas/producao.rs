//! Criação de Ordens de Produção a partir dos produtos marcados na lista de estoque.
//! Cada produto vira uma **Solicitação de Produção** (doc 02 §7.2) — o conceito que já existe no
//! motor, com prioridade, lead time e aprovação automática. Não há entidade nova aqui.
//!
//! Frontend burro (§3): a quantidade nasce da sugestão do motor; o usuário pode ajustar, mas nada
//! é recalculado aqui.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::api::criar_solicitacao;
use crate::componentes::{EstadoVazio, Icone, Seletor};
use crate::contexto::{ProdutoSelecionado, SelecaoProducao, Sessao};
use crate::formato::fmt_milhar;

/// Resultado da criação em lote, por produto.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Resultado {
    codigo: String,
    nome: String,
    erro: Option<String>,
}

#[component]
pub fn NovaProducao() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let selecao = expect_context::<SelecaoProducao>();

    // Quantidade por produto (nasce da sugestão do motor) e prioridade única do lote.
    let quantidades = RwSignal::new(Vec::<(String, i64)>::new());
    let prioridade = RwSignal::new("media".to_owned());
    let justificativa = RwSignal::new(String::new());
    let enviando = RwSignal::new(false);
    let resultados = RwSignal::new(Vec::<Resultado>::new());

    // Sincroniza as quantidades com a seleção (produto marcado depois entra com a sugestão).
    Effect::new(move |_| {
        let atuais = selecao.0.get();
        quantidades.update(|q| {
            q.retain(|(c, _)| atuais.iter().any(|p| &p.codigo == c));
            for p in &atuais {
                if !q.iter().any(|(c, _)| c == &p.codigo) {
                    q.push((p.codigo.clone(), p.qtd_sugerida.max(0)));
                }
            }
        });
    });

    let qtd_de = move |codigo: &str| -> i64 {
        quantidades
            .read()
            .iter()
            .find(|(c, _)| c == codigo)
            .map_or(0, |(_, q)| *q)
    };
    let set_qtd = move |codigo: String, valor: i64| {
        quantidades.update(|q| {
            if let Some(e) = q.iter_mut().find(|(c, _)| *c == codigo) {
                e.1 = valor.max(0);
            }
        });
    };

    let criar = criar_ordens(Estado {
        sessao,
        selecao,
        quantidades,
        prioridade,
        justificativa,
        enviando,
        resultados,
    });

    view! {
        <section class="pagina">
            <header class="prod-cab">
                <A href="/estoque" attr:class="icone-btn-claro" attr:aria-label="Voltar ao estoque" attr:title="Voltar">
                    <Icone arquivo="seta-esquerda.svg" />
                </A>
                <div class="prod-cab__id">
                    <h1 class="pagina__titulo">"Ordens de produção"</h1>
                    <p class="prod-cab__sub">
                        "Cada produto vira uma solicitação de produção, com prazo e prioridade."
                    </p>
                </div>
            </header>

            <Show
                when=move || !selecao.0.read().is_empty() || !resultados.read().is_empty()
                fallback=move || {
                    view! {
                        <EstadoVazio
                            arte="empty-search.svg"
                            titulo="Nenhum produto selecionado"
                            descricao="Marque os produtos na lista de estoque para criar as ordens."
                        />
                    }
                }
            >
                {move || {
                    let feitos = resultados.get();
                    if feitos.is_empty() {
                        view! { <FormularioOrdens
                            selecao
                            prioridade
                            justificativa
                            enviando
                            qtd_de
                            set_qtd
                            criar
                        /> }
                            .into_any()
                    } else {
                        view! { <ResumoCriacao feitos /> }.into_any()
                    }
                }}
            </Show>
        </section>
    }
}

/// Os sinais que o botão "Criar" precisa tocar. Agrupados para não passar sete argumentos soltos.
#[derive(Clone, Copy)]
struct Estado {
    sessao: Sessao,
    selecao: SelecaoProducao,
    quantidades: RwSignal<Vec<(String, i64)>>,
    prioridade: RwSignal<String>,
    justificativa: RwSignal<String>,
    enviando: RwSignal<bool>,
    resultados: RwSignal<Vec<Resultado>>,
}

/// Handler do "Criar": lê os sinais, dispara o lote e devolve o resultado por produto.
fn criar_ordens(e: Estado) -> impl Fn(leptos::ev::MouseEvent) + Copy + Send + Sync + 'static {
    move |_| {
        let Some(token) = e.sessao.0.get_untracked() else {
            return;
        };
        let itens = e.selecao.0.get_untracked();
        if itens.is_empty() || e.enviando.get_untracked() {
            return;
        }
        e.enviando.set(true);
        e.resultados.set(Vec::new());
        let lote = Lote {
            token,
            itens,
            quantidades: e.quantidades.get_untracked(),
            prioridade: e.prioridade.get_untracked(),
            justificativa: e.justificativa.get_untracked(),
        };
        leptos::task::spawn_local(async move {
            let saida = enviar_lote(lote).await;
            // Só os que FALHARAM continuam marcados — o usuário vê o que ficou pendente e pode
            // tentar de novo sem remontar a seleção inteira.
            let falhas: Vec<String> = saida
                .iter()
                .filter(|r| r.erro.is_some())
                .map(|r| r.codigo.clone())
                .collect();
            e.selecao
                .0
                .update(|v| v.retain(|p| falhas.contains(&p.codigo)));
            e.resultados.set(saida);
            e.enviando.set(false);
        });
    }
}

/// Um lote de ordens a criar (tudo já lido dos sinais — a tarefa async não toca em reativo).
struct Lote {
    token: String,
    itens: Vec<ProdutoSelecionado>,
    quantidades: Vec<(String, i64)>,
    prioridade: String,
    justificativa: String,
}

/// Cria uma Solicitação de Produção por produto, em sequência. Falha de uma NÃO aborta as demais:
/// cada produto tem seu resultado, e o usuário vê exatamente o que passou e o que não passou.
async fn enviar_lote(lote: Lote) -> Vec<Resultado> {
    let mut saida = Vec::with_capacity(lote.itens.len());
    for p in lote.itens {
        let qtd = lote
            .quantidades
            .iter()
            .find(|(c, _)| *c == p.codigo)
            .map_or(0, |(_, q)| *q);
        // Quantidade zero não vira ordem: seria uma ordem vazia.
        let erro = if qtd <= 0 {
            Some("quantidade zerada".to_owned())
        } else {
            criar_solicitacao(
                lote.token.clone(),
                p.codigo.clone(),
                qtd,
                lote.prioridade.clone(),
                lote.justificativa.clone(),
            )
            .await
            .err()
            .map(|e| e.to_string())
        };
        saida.push(Resultado {
            codigo: p.codigo,
            nome: p.nome,
            erro,
        });
    }
    saida
}

/// Lista os produtos marcados, com a quantidade (sugestão do motor, editável) e a prioridade.
#[component]
fn FormularioOrdens(
    selecao: SelecaoProducao,
    prioridade: RwSignal<String>,
    justificativa: RwSignal<String>,
    enviando: RwSignal<bool>,
    qtd_de: impl Fn(&str) -> i64 + Copy + Send + Sync + 'static,
    set_qtd: impl Fn(String, i64) + Copy + Send + Sync + 'static,
    criar: impl Fn(leptos::ev::MouseEvent) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let total = move || i64::try_from(selecao.0.read().len()).unwrap_or(0);
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">
                    {move || format!("{} produto(s) selecionado(s)", fmt_milhar(total()))}
                </h2>
                <button
                    type="button"
                    class="btn btn--secundario btn--sm"
                    on:click=move |_| selecao.limpar()
                >
                    "Limpar seleção"
                </button>
            </header>

            <div class="ordens-lista">
                <For
                    each=move || selecao.0.get()
                    key=|p: &ProdutoSelecionado| p.codigo.clone()
                    let(p)
                >
                    {
                        let codigo = p.codigo.clone();
                        let codigo_input = p.codigo.clone();
                        let codigo_rm = p.codigo.clone();
                        view! {
                            <div class="ordem-item">
                                <div class="ordem-item__id">
                                    <span class="ordem-item__nome">{p.nome.clone()}</span>
                                    <span class="ordem-item__cod">{p.codigo.clone()}</span>
                                </div>
                                <label class="ordem-item__qtd">
                                    <span class="texto-suave">"Quantidade"</span>
                                    <input
                                        class="input input--compacto"
                                        type="number"
                                        min="0"
                                        prop:value=move || qtd_de(&codigo).to_string()
                                        on:input=move |ev| {
                                            let v = event_target_value(&ev).parse::<i64>().unwrap_or(0);
                                            set_qtd(codigo_input.clone(), v);
                                        }
                                    />
                                </label>
                                <button
                                    type="button"
                                    class="icone-btn-claro"
                                    aria-label="Remover da seleção"
                                    on:click=move |_| {
                                        selecao
                                            .0
                                            .update(|v| v.retain(|x| x.codigo != codigo_rm));
                                    }
                                >
                                    <Icone arquivo="fechar.svg" />
                                </button>
                            </div>
                        }
                    }
                </For>
            </div>

            <footer class="ordens-rodape">
                <Seletor
                    icone="meta.svg"
                    rotulo="Prioridade"
                    opcoes=vec![("alta", "Alta"), ("media", "Média"), ("baixa", "Baixa")]
                    valor=Signal::derive(move || prioridade.get())
                    ao_escolher=move |v: String| prioridade.set(v)
                />
                <input
                    class="input input--compacto ordens-rodape__just"
                    placeholder="Justificativa (opcional)"
                    prop:value=move || justificativa.get()
                    on:input=move |ev| justificativa.set(event_target_value(&ev))
                />
                <button
                    type="button"
                    class="btn btn--escuro"
                    disabled=move || enviando.get().then_some("")
                    on:click=criar
                >
                    {move || {
                        if enviando.get() {
                            "Criando…".to_owned()
                        } else {
                            format!("Criar {} ordem(ns)", fmt_milhar(total()))
                        }
                    }}
                </button>
            </footer>
        </section>
    }
}

/// O que aconteceu com cada ordem — sem esconder falha.
#[component]
fn ResumoCriacao(feitos: Vec<Resultado>) -> impl IntoView {
    let ok = feitos.iter().filter(|r| r.erro.is_none()).count();
    let falhas = feitos.len() - ok;
    let linhas = feitos
        .iter()
        .map(|r| {
            let (classe, texto) = r.erro.as_ref().map_or_else(
                || ("badge badge--sev-positivo", "Criada".to_owned()),
                |e| ("badge badge--sev-critico", e.clone()),
            );
            view! {
                <div class="ordem-item">
                    <div class="ordem-item__id">
                        <span class="ordem-item__nome">{r.nome.clone()}</span>
                        <span class="ordem-item__cod">{r.codigo.clone()}</span>
                    </div>
                    <span class=classe>{texto}</span>
                </div>
            }
        })
        .collect_view();
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">
                    {format!("{} ordem(ns) criada(s)", fmt_milhar(i64::try_from(ok).unwrap_or(0)))}
                </h2>
                {(falhas > 0)
                    .then(|| {
                        view! {
                            <span class="badge badge--sev-critico">
                                {format!("{falhas} falhou(ram)")}
                            </span>
                        }
                    })}
            </header>
            <div class="ordens-lista">{linhas}</div>
            <footer class="ordens-rodape">
                <A href="/operacao" attr:class="btn btn--escuro">
                    "Ver na Operação"
                </A>
                <A href="/estoque" attr:class="btn btn--secundario">
                    "Voltar ao estoque"
                </A>
            </footer>
        </section>
    }
}
