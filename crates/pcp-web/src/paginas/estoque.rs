//! Gestão de Estoque (doc 03 §3): tabela de produtos ativos paginada NO SERVIDOR, com cards de
//! resumo clicáveis (aplicam filtro), busca, filtros de classe/status, ordenação e tamanho de
//! página. Frontend burro (CLAUDE.md §3): exibe valores já calculados pelo motor — nada é
//! recalculado aqui. Cobertura 999 vira "Sem histórico" e quantidades levam separador de
//! milhar (§12). Tempo real fica por conta do refresh pós-pipeline; há botão de atualizar.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::api::{
    estoque, exportar_estoque, obter_preferencias, painel, ConsultaEstoque, LinhaEstoque,
    PainelResumo,
};
use crate::contexto::Sessao;
use crate::download;
use crate::formato::{fmt_cobertura, fmt_milhar, nome_exibicao, rotulo_status};

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
    let cobertura_min = RwSignal::new(None::<f64>);
    let cobertura_max = RwSignal::new(None::<f64>);
    let apenas_sugestao = RwSignal::new(false);
    let apenas_fora_linha = RwSignal::new(false);
    let limite = RwSignal::new(50_i64);
    let deslocamento = RwSignal::new(0_i64);
    let tick = RwSignal::new(0_u32);

    // Tamanho de página inicial = preferência do usuário (doc 03 §8), aplicada ao carregar.
    let prefs = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => obter_preferencias(t).await.ok(),
                None => None,
            }
        },
    );
    Effect::new(move |_| {
        if let Some(Some(p)) = prefs.get() {
            limite.set(i64::from(p.tamanho_pagina));
        }
    });

    // Qualquer mudança de filtro volta para a primeira página.
    let resetar = move || deslocamento.set(0);

    // Consulta atual a partir dos sinais. Para o servidor inteira (filtros + faixas + switches).
    let consulta_atual = move || ConsultaEstoque {
        classe: classe.get(),
        status: status.get(),
        busca: Some(busca.get()),
        ordem: Some(ordem.get()),
        cobertura_min: cobertura_min.get(),
        cobertura_max: cobertura_max.get(),
        apenas_sugestao: apenas_sugestao.get(),
        apenas_fora_linha: apenas_fora_linha.get(),
        limite: limite.get(),
        deslocamento: deslocamento.get(),
    };

    // Exporta o filtro atual inteiro (CSV/JSON) e dispara o download no cliente (§12).
    let exportar = move |formato: &'static str| {
        let Some(token) = sessao.0.get_untracked() else {
            return;
        };
        let consulta = untrack(consulta_atual);
        let nome = if formato == "json" {
            "estoque.json"
        } else {
            "estoque.csv"
        };
        leptos::task::spawn_local(async move {
            match exportar_estoque(token, consulta, formato.to_owned()).await {
                Ok(conteudo) => download::baixar(nome, &conteudo),
                Err(e) => leptos::logging::error!("exportação falhou: {e}"),
            }
        });
    };

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
        move || (sessao.0.get(), consulta_atual(), tick.get()),
        |(token, consulta, _)| async move {
            match token {
                Some(t) => estoque(t, consulta).await,
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
                            Ok(p) => {
                                view! {
                                    {kpis_estoque(&p)}
                                    {abas_classe(&p, classe, resetar)}
                                }
                                    .into_any()
                            }
                            Err(_) => ().into_any(),
                        })
                }}
            </Suspense>

            <Filtros
                status
                busca
                busca_input
                ordem
                cobertura_min
                cobertura_max
                apenas_sugestao
                apenas_fora_linha
                resetar
                exportar
            />

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
                                    <div class="tabela-cartao">
                                        <Tabela itens=pag.itens />
                                        <Paginacao limite deslocamento total />
                                    </div>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </section>
    }
}

/// KPIs do estoque (métricas reais do painel — frontend burro, §3). Sem dimensão financeira
/// (custo/preço adiados — §6) nem "giro" (não calculado pelo motor): só o que a API já entrega.
fn kpis_estoque(p: &PainelResumo) -> impl IntoView {
    let abaixo: i64 = p
        .por_status
        .iter()
        .filter(|c| {
            matches!(
                c.rotulo.as_str(),
                "critico" | "sem_estoque" | "estoque_baixo" | "baixo"
            )
        })
        .map(|c| c.quantidade)
        .sum();
    let cobertura = p
        .cobertura_media
        .map_or_else(|| "—".to_owned(), |c| format!("{c:.0}"));
    view! {
        <div class="kpis">
            <KpiEstoque
                icone="inventory.svg"
                valor=fmt_milhar(p.total_produtos)
                rotulo="Produtos ativos"
            />
            <KpiEstoque
                icone="alerta.svg"
                valor=fmt_milhar(abaixo)
                rotulo="Abaixo do recomendado"
            />
            <KpiEstoque icone="relogio.svg" valor=cobertura rotulo="Cobertura média" />
            <KpiEstoque
                icone="orders.svg"
                valor=fmt_milhar(p.total_sugerido)
                rotulo="A produzir"
            />
        </div>
    }
}

#[component]
fn KpiEstoque(icone: &'static str, valor: String, rotulo: &'static str) -> impl IntoView {
    let estilo = format!("-webkit-mask-image:url(/icons/{icone});mask-image:url(/icons/{icone})");
    view! {
        <div class="kpi kpi--icone">
            <span class="kpi__chip">
                <span class="icone-mask" style=estilo></span>
            </span>
            <span class="kpi__valor">{valor}</span>
            <span class="kpi__rotulo">{rotulo}</span>
        </div>
    }
}

/// Abas de filtro por classe ABC (substituem as categorias de material do mockup — no PCP a
/// categorização canônica é a classe A/B/C/D/F/N, §4). Contagens reais do painel.
fn abas_classe(
    p: &PainelResumo,
    classe: RwSignal<Option<String>>,
    resetar: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let mut presentes: Vec<(String, i64)> = p
        .por_classe
        .iter()
        .map(|c| (c.rotulo.clone(), c.quantidade))
        .collect();
    presentes.sort_by_key(|(c, _)| ordem_classe(c));
    view! {
        <div class="abas-classe">
            <AbaClasse classe rotulo="Todas".to_owned() valor=None contagem=p.total_produtos resetar />
            {presentes
                .into_iter()
                .map(|(cod, qtd)| {
                    view! {
                        <AbaClasse classe rotulo=cod.clone() valor=Some(cod) contagem=qtd resetar />
                    }
                })
                .collect_view()}
        </div>
    }
}

/// Ordem canônica das classes para exibição das abas (A→N).
fn ordem_classe(c: &str) -> u8 {
    match c {
        "A" => 0,
        "B" => 1,
        "C" => 2,
        "D" => 3,
        "F" => 4,
        "N" => 5,
        _ => 9,
    }
}

#[component]
fn AbaClasse(
    classe: RwSignal<Option<String>>,
    rotulo: String,
    valor: Option<String>,
    contagem: i64,
    resetar: impl Fn() + Copy + 'static,
) -> impl IntoView {
    let alvo = valor.clone();
    let ativo = {
        let alvo = alvo.clone();
        move || classe.get() == alvo
    };
    view! {
        <button
            type="button"
            class="aba"
            class:aba--ativa=ativo
            on:click=move |_| {
                classe.set(alvo.clone());
                resetar();
            }
        >
            <span>{rotulo}</span>
            <span class="aba__contagem">{fmt_milhar(contagem)}</span>
        </button>
    }
}

#[component]
#[allow(clippy::too_many_lines)] // markup declarativo dos filtros (uma responsabilidade)
fn Filtros(
    status: RwSignal<Option<String>>,
    busca: RwSignal<String>,
    busca_input: RwSignal<String>,
    ordem: RwSignal<String>,
    cobertura_min: RwSignal<Option<f64>>,
    cobertura_max: RwSignal<Option<f64>>,
    apenas_sugestao: RwSignal<bool>,
    apenas_fora_linha: RwSignal<bool>,
    resetar: impl Fn() + Copy + 'static,
    exportar: impl Fn(&'static str) + Copy + 'static,
) -> impl IntoView {
    let aplicar_busca = move || {
        busca.set(busca_input.get());
        resetar();
    };
    // Lê um campo numérico não-negativo (vazio → sem limite).
    let parse_cobertura = |valor: String| valor.parse::<f64>().ok().filter(|n| *n >= 0.0);
    let estilo_export =
        "-webkit-mask-image:url(/icons/exportar.svg);mask-image:url(/icons/exportar.svg)";
    view! {
        <div class="estoque-filtros">
            <div class="estoque-filtros__linha">
                <form
                    class="estoque-filtros__busca"
                    on:submit=move |ev| {
                        ev.prevent_default();
                        aplicar_busca();
                    }
                >
                    <input
                        class="input"
                        placeholder="Buscar item, código, SKU…"
                        prop:value=move || busca_input.get()
                        on:input=move |ev| busca_input.set(event_target_value(&ev))
                    />
                    <button type="submit" class="btn btn--escuro">
                        "Buscar"
                    </button>
                </form>

                <span class="toolbar-sep"></span>

                <select
                    class="select"
                    aria-label="Ordenar"
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
                <select
                    class="select"
                    aria-label="Status"
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
                    <option value="fora_de_linha">"Fora de Linha"</option>
                </select>

                <details class="export-menu">
                    <summary class="export-menu__btn" aria-label="Exportar filtro completo">
                        <span class="icone-mask" style=estilo_export></span>
                    </summary>
                    <div class="export-menu__lista">
                        <button type="button" on:click=move |_| exportar("csv")>
                            "Exportar CSV"
                        </button>
                        <button type="button" on:click=move |_| exportar("json")>
                            "Exportar JSON"
                        </button>
                    </div>
                </details>
            </div>

            <div class="estoque-filtros__avancado">
                <div class="faixa">
                    <span class="campo-select__rotulo">"Cobertura (dias)"</span>
                    <input
                        class="input input--num"
                        type="number"
                        min="0"
                        placeholder="mín"
                        prop:value=move || {
                            cobertura_min.get().map(|n| n.to_string()).unwrap_or_default()
                        }
                        on:input=move |ev| {
                            cobertura_min.set(parse_cobertura(event_target_value(&ev)));
                            resetar();
                        }
                    />
                    <span class="faixa__ate">"até"</span>
                    <input
                        class="input input--num"
                        type="number"
                        min="0"
                        placeholder="máx"
                        prop:value=move || {
                            cobertura_max.get().map(|n| n.to_string()).unwrap_or_default()
                        }
                        on:input=move |ev| {
                            cobertura_max.set(parse_cobertura(event_target_value(&ev)));
                            resetar();
                        }
                    />
                </div>
                <label class="switch">
                    <input
                        type="checkbox"
                        prop:checked=move || apenas_sugestao.get()
                        on:change=move |ev| {
                            apenas_sugestao.set(event_target_checked(&ev));
                            resetar();
                        }
                    />
                    <span>"Apenas com sugestão"</span>
                </label>
                <label class="switch">
                    <input
                        type="checkbox"
                        prop:checked=move || apenas_fora_linha.get()
                        on:change=move |ev| {
                            apenas_fora_linha.set(event_target_checked(&ev));
                            resetar();
                        }
                    />
                    <span>"Apenas fora de linha"</span>
                </label>
            </div>
        </div>
    }
}

#[component]
fn Tabela(itens: Vec<LinhaEstoque>) -> impl IntoView {
    view! {
        <div class="tabela-rolavel">
            <table class="tabela tabela--centro">
                <thead>
                    <tr>
                        <th>"Código"</th>
                        <th>"Item"</th>
                        <th>"Classe"</th>
                        <th class="tabela__nivel-col">"Nível de estoque"</th>
                        <th class="tabela__num">"Disponível"</th>
                        <th class="tabela__num">"Produzir"</th>
                        <th>"Status"</th>
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

/// Cor do semáforo de status (§12) usada na barra de nível. Espelha os badges de status.
fn cor_status(status: &str) -> &'static str {
    match status {
        "critico" | "sem_estoque" => "var(--semaforo-critico)",
        "estoque_baixo" => "var(--semaforo-alto)",
        "baixo" => "var(--semaforo-medio)",
        "adequado" => "var(--semaforo-ok)",
        "alto" | "excessivo" => "var(--semaforo-info)",
        _ => "var(--abc-d)",
    }
}

#[component]
#[allow(clippy::cast_precision_loss)] // quantidades de estoque: conversão exata para f64 na razão
fn Linha(i: LinhaEstoque) -> impl IntoView {
    let nome = nome_exibicao(
        i.produto.as_deref(),
        i.configuracao.as_deref(),
        &i.codigo_estoque,
    );
    let href = format!("/estoque/{}", i.codigo_estoque);
    let classe_abc = format!(
        "badge badge--circulo badge--abc-{}",
        i.classe.to_lowercase()
    );
    let cor_st = cor_status(&i.status);
    // Barra de nível: preenchimento = disponível / recomendado (0–100%), cor pelo status.
    // O alvo (recomendado) vai no rótulo; nada é recalculado aqui — só visualização (§3).
    let alvo = i.estoque_total_recomendado.max(1);
    let pct = ((i.qtd_disponivel as f64 / alvo as f64) * 100.0).clamp(0.0, 100.0);
    let estilo_barra = format!("width:{pct:.0}%;background:{cor_st}");
    let recomendada = i.estoque_total_recomendado;
    view! {
        <tr>
            <td class="tabela__cod">{i.codigo_estoque.clone()}</td>
            <td>
                <A href=href attr:class="tabela__produto-link">
                    <span class="tabela__nome">{nome}</span>
                    {i.sku
                        .clone()
                        .filter(|s| !s.is_empty())
                        .map(|s| view! { <span class="tabela__sku">{s}</span> })}
                </A>
            </td>
            <td>
                <span class=classe_abc>{i.classe.clone()}</span>
            </td>
            <td class="tabela__nivel-col">
                <div class="nivel">
                    <div class="nivel__trilho">
                        <span class="nivel__preenche" style=estilo_barra></span>
                    </div>
                    <span class="nivel__ref">{format!("rec. {} un", fmt_milhar(recomendada))}</span>
                </div>
            </td>
            <td class="tabela__num">
                <div class="tabela__disp">
                    <span class="tabela__disp-valor">{format!("{} un", fmt_milhar(i.qtd_disponivel))}</span>
                    <span class="tabela__disp-cob">
                        {format!("{} de cobertura", fmt_cobertura(i.cobertura_dias))}
                    </span>
                </div>
            </td>
            <td class="tabela__num tabela__produzir">
                {if i.qtd_sugerida > 0 {
                    format!("{} un", fmt_milhar(i.qtd_sugerida))
                } else {
                    "—".to_owned()
                }}
            </td>
            <td class="tabela__status">
                <span class="status-texto" style=format!("color:{cor_st}")>
                    {rotulo_status(&i.status)}
                </span>
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
                {move || {
                    if total == 0 {
                        "Nenhum item".to_owned()
                    } else {
                        format!(
                            "Mostrando {}–{} de {} itens",
                            fmt_milhar(inicio()),
                            fmt_milhar(fim()),
                            fmt_milhar(total),
                        )
                    }
                }}
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
