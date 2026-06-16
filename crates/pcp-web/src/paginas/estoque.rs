//! Gestão de Estoque (doc 03 §3): tabela de produtos ativos paginada NO SERVIDOR, com cards de
//! resumo clicáveis (aplicam filtro), busca, filtros de classe/status, ordenação e tamanho de
//! página. Frontend burro (CLAUDE.md §3): exibe valores já calculados pelo motor — nada é
//! recalculado aqui. Cobertura 999 vira "Sem histórico" e quantidades levam separador de
//! milhar (§12). Tempo real fica por conta do refresh pós-pipeline; há botão de atualizar.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::api::{estoque, painel, ConsultaEstoque, LinhaEstoque, PainelResumo};
use crate::contexto::Sessao;

/// Nome de exibição "{produto} - {cor}" — cor = texto após ':' da configuração (doc 02 §10/§12).
fn nome_exibicao(l: &LinhaEstoque) -> String {
    let produto = l
        .produto
        .clone()
        .unwrap_or_else(|| l.codigo_estoque.clone());
    match l.configuracao.as_deref().and_then(|c| c.split(':').nth(1)) {
        Some(cor) if !cor.trim().is_empty() => format!("{produto} - {}", cor.trim()),
        _ => produto,
    }
}

/// Inteiro com separador de milhar à brasileira (§12): `1234567` → `1.234.567`.
fn fmt_milhar(n: i64) -> String {
    let negativo = n < 0;
    let digitos = n.unsigned_abs().to_string();
    let n_dig = digitos.len();
    let mut saida = String::with_capacity(n_dig + n_dig / 3 + 1);
    for (i, ch) in digitos.chars().enumerate() {
        if i != 0 && (n_dig - i).is_multiple_of(3) {
            saida.push('.');
        }
        saida.push(ch);
    }
    if negativo {
        format!("-{saida}")
    } else {
        saida
    }
}

/// Cobertura: sentinela 999 vira "Sem histórico" (§12); senão 1 casa decimal.
fn fmt_cobertura(c: f64) -> String {
    if c >= 999.0 {
        "Sem histórico".to_owned()
    } else {
        format!("{c:.1}")
    }
}

fn rotulo_status(codigo: &str) -> &'static str {
    match codigo {
        "sem_estoque" => "Sem estoque",
        "fora_de_linha" => "Fora de linha",
        "sem_historico" => "Sem histórico",
        "critico" => "Crítico",
        "estoque_baixo" => "Estoque baixo",
        "baixo" => "Baixo",
        "adequado" => "Adequado",
        "alto" => "Alto",
        "excessivo" => "Excessivo",
        _ => "—",
    }
}

#[component]
#[allow(clippy::too_many_lines)] // a maior parte é markup declarativo (view!), não lógica
pub fn PaginaEstoque() -> impl IntoView {
    let sessao = expect_context::<Sessao>();

    // Filtros e paginação (estado do cliente; a consulta vai inteira para o servidor).
    let classe = RwSignal::new(None::<String>);
    let status = RwSignal::new(None::<String>);
    let busca = RwSignal::new(String::new()); // termo aplicado
    let busca_input = RwSignal::new(String::new()); // o que está sendo digitado
    let ordem = RwSignal::new("sugerida_desc".to_owned());
    let limite = RwSignal::new(50_i64);
    let deslocamento = RwSignal::new(0_i64);
    let tick = RwSignal::new(0_u32);

    // Qualquer mudança de filtro volta para a primeira página.
    let resetar = move || deslocamento.set(0);

    let painel_res = Resource::new(
        move || (sessao.0.get(), tick.get()),
        |(t, _)| async move {
            match t {
                Some(t) => painel(t).await,
                None => Ok(PainelResumo::default()),
            }
        },
    );

    let dados = Resource::new(
        move || {
            (
                sessao.0.get(),
                classe.get(),
                status.get(),
                busca.get(),
                ordem.get(),
                limite.get(),
                deslocamento.get(),
                tick.get(),
            )
        },
        |(token, classe, status, busca, ordem, limite, deslocamento, _)| async move {
            match token {
                Some(t) => {
                    estoque(
                        t,
                        ConsultaEstoque {
                            classe,
                            status,
                            busca: Some(busca),
                            ordem: Some(ordem),
                            limite,
                            deslocamento,
                        },
                    )
                    .await
                }
                None => Ok(crate::api::PaginaEstoque::default()),
            }
        },
    );

    view! {
        <section class="pagina">
            <header class="pagina__cab">
                <h1 class="pagina__titulo">"Gestão de Estoque"</h1>
                <p class="texto-suave">
                    "Produtos ativos — situação, cobertura e o que produzir."
                </p>
            </header>

            <Suspense fallback=|| {
                view! { <p class="texto-suave">"Carregando resumo…"</p> }
            }>
                {move || {
                    painel_res
                        .get()
                        .map(|res| match res {
                            Ok(p) => cards_resumo(&p, status, resetar).into_any(),
                            Err(_) => ().into_any(),
                        })
                }}
            </Suspense>

            <Filtros classe status busca busca_input ordem limite resetar />

            <Suspense fallback=|| {
                view! { <p class="texto-suave">"Carregando produtos…"</p> }
            }>
                {move || {
                    dados
                        .get()
                        .map(|res| match res {
                            Err(e) => {
                                view! { <p class="form-auth__erro">{e.to_string()}</p> }.into_any()
                            }
                            Ok(pag) if pag.itens.is_empty() => {
                                view! {
                                    <p class="estado-vazio">"Nenhum produto para os filtros atuais."</p>
                                }
                                    .into_any()
                            }
                            Ok(pag) => {
                                let total = pag.total;
                                view! {
                                    <Tabela itens=pag.itens />
                                    <Paginacao limite deslocamento total />
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </section>
    }
}

/// Cards de resumo clicáveis: Total (limpa) + um por status presente (aplica o filtro). Doc 03 §3.
fn cards_resumo(
    p: &PainelResumo,
    status: RwSignal<Option<String>>,
    resetar: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let total: i64 = p.por_status.iter().map(|c| c.quantidade).sum();
    let cards: Vec<_> = p
        .por_status
        .iter()
        .map(|c| {
            let codigo = c.rotulo.clone();
            (rotulo_status(&codigo), codigo, c.quantidade)
        })
        .collect();

    view! {
        <div class="cards-resumo">
            <CartaResumo
                rotulo="Total"
                valor=total
                codigo=None
                status
                resetar
            />
            {cards
                .into_iter()
                .map(|(rot, codigo, qtd)| {
                    view! {
                        <CartaResumo
                            rotulo=rot
                            valor=qtd
                            codigo=Some(codigo)
                            status
                            resetar
                        />
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn CartaResumo(
    rotulo: &'static str,
    valor: i64,
    codigo: Option<String>,
    status: RwSignal<Option<String>>,
    resetar: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let alvo = codigo.clone();
    let ativo = {
        let alvo = alvo.clone();
        move || status.get() == alvo
    };
    view! {
        <button
            type="button"
            class="carta-resumo carta-resumo--clicavel"
            class:carta-resumo--ativa=ativo
            on:click=move |_| {
                status.set(alvo.clone());
                resetar();
            }
        >
            <span class="carta-resumo__valor">{fmt_milhar(valor)}</span>
            <span class="carta-resumo__rotulo">{rotulo}</span>
        </button>
    }
}

#[component]
fn Filtros(
    classe: RwSignal<Option<String>>,
    status: RwSignal<Option<String>>,
    busca: RwSignal<String>,
    busca_input: RwSignal<String>,
    ordem: RwSignal<String>,
    limite: RwSignal<i64>,
    resetar: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let aplicar_busca = move || {
        busca.set(busca_input.get());
        resetar();
    };
    view! {
        <div class="filtros-estoque">
            <form
                class="filtros-estoque__busca"
                on:submit=move |ev| {
                    ev.prevent_default();
                    aplicar_busca();
                }
            >
                <input
                    class="input"
                    placeholder="Buscar por código, produto ou SKU…"
                    prop:value=move || busca_input.get()
                    on:input=move |ev| busca_input.set(event_target_value(&ev))
                />
                <button type="submit" class="btn btn--secundario">
                    "Buscar"
                </button>
            </form>

            <div class="chips">
                <ChipClasse classe rotulo="Todas" valor=None resetar />
                <ChipClasse classe rotulo="A" valor=Some("A") resetar />
                <ChipClasse classe rotulo="B" valor=Some("B") resetar />
                <ChipClasse classe rotulo="C" valor=Some("C") resetar />
                <ChipClasse classe rotulo="D" valor=Some("D") resetar />
                <ChipClasse classe rotulo="F" valor=Some("F") resetar />
                <ChipClasse classe rotulo="N" valor=Some("N") resetar />
            </div>

            <div class="filtros-estoque__selects">
                <label class="campo-select">
                    <span class="campo-select__rotulo">"Ordenar"</span>
                    <select
                        class="select"
                        on:change=move |ev| {
                            ordem.set(event_target_value(&ev));
                            resetar();
                        }
                        prop:value=move || ordem.get()
                    >
                        <option value="sugerida_desc">"Sugestão (maior)"</option>
                        <option value="cobertura_asc">"Cobertura (menor)"</option>
                        <option value="cobertura_desc">"Cobertura (maior)"</option>
                        <option value="disponivel_desc">"Disponível (maior)"</option>
                        <option value="disponivel_asc">"Disponível (menor)"</option>
                        <option value="recomendada_desc">"Recomendada (maior)"</option>
                        <option value="produto_asc">"Produto (A–Z)"</option>
                        <option value="produto_desc">"Produto (Z–A)"</option>
                        <option value="classe_asc">"Classe (A→N)"</option>
                    </select>
                </label>
                <label class="campo-select">
                    <span class="campo-select__rotulo">"Status"</span>
                    <select
                        class="select"
                        on:change=move |ev| {
                            let v = event_target_value(&ev);
                            status.set((!v.is_empty()).then_some(v));
                            resetar();
                        }
                        prop:value=move || status.get().unwrap_or_default()
                    >
                        <option value="">"Todos"</option>
                        <option value="critico">"Crítico"</option>
                        <option value="sem_estoque">"Sem estoque"</option>
                        <option value="estoque_baixo">"Estoque baixo"</option>
                        <option value="baixo">"Baixo"</option>
                        <option value="adequado">"Adequado"</option>
                        <option value="alto">"Alto"</option>
                        <option value="excessivo">"Excessivo"</option>
                        <option value="sem_historico">"Sem histórico"</option>
                        <option value="fora_de_linha">"Fora de linha"</option>
                    </select>
                </label>
                <label class="campo-select">
                    <span class="campo-select__rotulo">"Por página"</span>
                    <select
                        class="select"
                        on:change=move |ev| {
                            if let Ok(v) = event_target_value(&ev).parse::<i64>() {
                                limite.set(v);
                                resetar();
                            }
                        }
                        prop:value=move || limite.get().to_string()
                    >
                        <option value="50">"50"</option>
                        <option value="100">"100"</option>
                        <option value="500">"500"</option>
                        <option value="1000">"1000"</option>
                    </select>
                </label>
            </div>
        </div>
    }
}

#[component]
fn ChipClasse(
    classe: RwSignal<Option<String>>,
    rotulo: &'static str,
    valor: Option<&'static str>,
    resetar: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let ativo = move || classe.get().as_deref() == valor;
    view! {
        <button
            type="button"
            class="chip"
            class:chip--ativo=ativo
            on:click=move |_| {
                classe.set(valor.map(ToOwned::to_owned));
                resetar();
            }
        >
            {rotulo}
        </button>
    }
}

#[component]
fn Tabela(itens: Vec<LinhaEstoque>) -> impl IntoView {
    view! {
        <div class="tabela-rolavel">
            <table class="tabela">
                <thead>
                    <tr>
                        <th>"Código"</th>
                        <th>"Produto"</th>
                        <th>"Classe"</th>
                        <th class="tabela__num">"Disponível"</th>
                        <th class="tabela__num">"Média/dia"</th>
                        <th class="tabela__num">"Cobertura"</th>
                        <th class="tabela__num">"Mínimo"</th>
                        <th class="tabela__num">"Recomendada"</th>
                        <th>"Status"</th>
                        <th class="tabela__num">"Produzir"</th>
                        <th></th>
                    </tr>
                </thead>
                <tbody>
                    {itens
                        .into_iter()
                        .map(|i| view! { <Linha i /> })
                        .collect_view()}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn Linha(i: LinhaEstoque) -> impl IntoView {
    let nome = nome_exibicao(&i);
    let href = format!("/estoque/{}", i.codigo_estoque);
    let classe_abc = format!("badge badge--abc-{}", i.classe.to_lowercase());
    let classe_status = format!("badge badge--status-{}", i.status);
    view! {
        <tr>
            <td class="tabela__cod">{i.codigo_estoque.clone()}</td>
            <td>
                <div class="tabela__produto">
                    <span class="tabela__nome">{nome}</span>
                    {i.sku
                        .clone()
                        .filter(|s| !s.is_empty())
                        .map(|s| view! { <span class="tabela__sku">{s}</span> })}
                </div>
            </td>
            <td>
                <span class=classe_abc>{i.classe.clone()}</span>
            </td>
            <td class="tabela__num">{fmt_milhar(i.qtd_disponivel)}</td>
            <td class="tabela__num">{format!("{:.1}", i.media_diaria)}</td>
            <td class="tabela__num">{fmt_cobertura(i.cobertura_dias)}</td>
            <td class="tabela__num">{fmt_milhar(i.estoque_minimo)}</td>
            <td class="tabela__num">{fmt_milhar(i.estoque_total_recomendado)}</td>
            <td>
                <span class=classe_status>{rotulo_status(&i.status)}</span>
            </td>
            <td class="tabela__num tabela__produzir">{fmt_milhar(i.qtd_sugerida)}</td>
            <td>
                <A href=href attr:class="btn btn--secundario btn--sm">
                    "Ver detalhes"
                </A>
            </td>
        </tr>
    }
}

#[component]
fn Paginacao(limite: RwSignal<i64>, deslocamento: RwSignal<i64>, total: i64) -> impl IntoView {
    let inicio = move || {
        if total == 0 {
            0
        } else {
            deslocamento.get() + 1
        }
    };
    let fim = move || (deslocamento.get() + limite.get()).min(total);
    let tem_anterior = move || deslocamento.get() > 0;
    let tem_proximo = move || deslocamento.get() + limite.get() < total;

    view! {
        <nav class="paginacao">
            <span class="paginacao__info">
                {move || format!("{}–{} de {}", fmt_milhar(inicio()), fmt_milhar(fim()), fmt_milhar(total))}
            </span>
            <div class="paginacao__botoes">
                <button
                    type="button"
                    class="btn btn--secundario btn--sm"
                    prop:disabled=move || !tem_anterior()
                    on:click=move |_| {
                        deslocamento.update(|d| *d = (*d - limite.get()).max(0));
                    }
                >
                    "Anterior"
                </button>
                <button
                    type="button"
                    class="btn btn--secundario btn--sm"
                    prop:disabled=move || !tem_proximo()
                    on:click=move |_| {
                        deslocamento.update(|d| *d += limite.get());
                    }
                >
                    "Próxima"
                </button>
            </div>
        </nav>
    }
}
