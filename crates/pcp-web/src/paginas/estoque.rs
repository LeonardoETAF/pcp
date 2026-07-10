//! Gestão de Estoque (doc 03 §3): tabela de produtos ativos paginada NO SERVIDOR, com cards de
//! resumo clicáveis (aplicam filtro), busca, filtros de classe/status, ordenação e tamanho de
//! página. Frontend burro (CLAUDE.md §3): exibe valores já calculados pelo motor — nada é
//! recalculado aqui. Cobertura 999 vira "Sem histórico" e quantidades levam separador de
//! milhar (§12). Tempo real fica por conta do refresh pós-pipeline; há botão de atualizar.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::api::{
    estoque, exportar_estoque, obter_preferencias, painel, ConsultaEstoque, ContagemClasse,
    LinhaEstoque, PainelResumo,
};
use crate::componentes::{EstadoVazio, Icone, Seletor};
use crate::contexto::Sessao;
use crate::download;
use crate::erro::mensagem_usuario;
use crate::formato::{cor_partes, fmt_cobertura, fmt_milhar, rotulo_status};

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

    // Consulta atual a partir dos sinais; vai inteira para o servidor. A API ainda aceita faixa de
    // cobertura e os recortes "só com sugestão"/"só fora de linha", mas a tela não os expõe: aqui
    // seguem nos valores neutros, que não restringem o resultado.
    let consulta_atual = move || ConsultaEstoque {
        classe: classe.get(),
        status: status.get(),
        busca: Some(busca.get()),
        ordem: Some(ordem.get()),
        cobertura_min: None,
        cobertura_max: None,
        apenas_sugestao: false,
        apenas_fora_linha: false,
        limite: limite.get(),
        deslocamento: deslocamento.get(),
    };

    // Exporta o filtro atual inteiro e dispara o download no cliente (§12). A API também serve
    // JSON, mas a tela oferece só o CSV (UTF-8 com BOM), que é o que o Excel BR abre direto.
    let exportar = move || {
        let Some(token) = sessao.0.get_untracked() else {
            return;
        };
        let consulta = untrack(consulta_atual);
        leptos::task::spawn_local(async move {
            match exportar_estoque(token, consulta, "csv".to_owned()).await {
                Ok(conteudo) => download::baixar("estoque.csv", &conteudo),
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
            <Suspense fallback=|| {
                view! { <p class="texto-suave">"Carregando resumo…"</p> }
            }>
                {move || {
                    painel_res.get().map(|res| match res {
                        Ok(p) => kpis_estoque(&p).into_any(),
                        Err(_) => ().into_any(),
                    })
                }}
            </Suspense>

            // Busca e classes formam UM card: são o mesmo filtro, aplicado em duas linhas.
            <div class="estoque-filtros">
                <Filtros status busca busca_input ordem resetar exportar />
                // As contagens vêm da MESMA consulta da lista: mudam com a busca e o status,
                // e nunca dessincronizam do que a tabela mostra.
                <Suspense fallback=|| ()>
                    {move || {
                        dados.get().map(|res| match res {
                            Ok(pag) => abas_classe(&pag.contagem_classes, classe, resetar).into_any(),
                            Err(_) => ().into_any(),
                        })
                    }}
                </Suspense>
            </div>

            <Suspense fallback=|| {
                view! { <p class="texto-suave">"Carregando produtos…"</p> }
            }>
                {move || {
                    dados
                        .get()
                        .map(|res| match res {
                            Err(e) => {
                                view! { <p class="form-auth__erro">{mensagem_usuario(&e)}</p> }.into_any()
                            }
                            Ok(pag) if pag.itens.is_empty() => {
                                view! {
                                    <EstadoVazio
                                        arte="empty-search.svg"
                                        titulo="Nenhum produto para os filtros atuais"
                                        descricao="Ajuste a busca, a classe ou o status para ver resultados."
                                    />
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

/// Traço exibido quando o motor não tem valor para o KPI: sem número, não há unidade a mostrar.
const SEM_VALOR: &str = "—";

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
        .map_or_else(|| SEM_VALOR.to_owned(), |c| format!("{c:.0}"));
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
            <KpiEstoque
                icone="relogio.svg"
                valor=cobertura
                rotulo="Cobertura média"
                unidade="dias"
            />
            <KpiEstoque
                icone="orders.svg"
                valor=fmt_milhar(p.total_sugerido)
                rotulo="A produzir"
            />
        </div>
    }
}

/// KPI horizontal: ícone ao lado do texto (o card fica mais baixo que na versão em coluna).
#[component]
fn KpiEstoque(
    icone: &'static str,
    valor: String,
    rotulo: &'static str,
    /// Unidade ao lado do número (ex.: "dias"). Omitida quando não há valor a qualificar.
    #[prop(optional)]
    unidade: Option<&'static str>,
) -> impl IntoView {
    let com_unidade = unidade.filter(|_| valor != SEM_VALOR);
    view! {
        <div class="kpi kpi--linha">
            <span class="kpi__chip">
                <Icone arquivo=icone />
            </span>
            <div class="kpi__corpo">
                <span class="kpi__valor">
                    {valor}
                    {com_unidade.map(|u| view! { <span class="kpi__unidade">{u}</span> })}
                </span>
                <span class="kpi__rotulo">{rotulo}</span>
            </div>
        </div>
    }
}

/// Abas de filtro por classe ABC (§4). As contagens são as da consulta corrente (busca + status),
/// não as do catálogo inteiro: cada botão diz quantos itens traria se fosse o escolhido.
fn abas_classe(
    contagens: &[ContagemClasse],
    classe: RwSignal<Option<String>>,
    resetar: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let mut presentes: Vec<(String, i64)> = contagens
        .iter()
        .map(|c| (c.classe.clone(), c.quantidade))
        .collect();
    presentes.sort_by_key(|(c, _)| ordem_classe(c));
    // "Todos" é a soma das classes sob o mesmo filtro — não o total do catálogo.
    let total: i64 = presentes.iter().map(|(_, q)| q).sum();
    view! {
        <div class="estoque-filtros__classes">
            <AbaClasse classe rotulo="Todos".to_owned() valor=None contagem=total resetar />
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
    resetar: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let alvo = valor.clone();
    let ativo = Memo::new(move |_| classe.get() == valor);
    view! {
        <button
            type="button"
            class="aba-classe"
            class:aba-classe--ativa=move || ativo.get()
            aria-pressed=move || if ativo.get() { "true" } else { "false" }
            on:click=move |_| {
                classe.set(alvo.clone());
                resetar();
            }
        >
            <span class="aba-classe__rotulo">{rotulo}</span>
            <span class="aba-classe__contagem">{fmt_milhar(contagem)}</span>
        </button>
    }
}

/// Primeira linha do card de filtros: busca, ordenação, status e exportação.
#[component]
fn Filtros(
    status: RwSignal<Option<String>>,
    busca: RwSignal<String>,
    busca_input: RwSignal<String>,
    ordem: RwSignal<String>,
    resetar: impl Fn() + Copy + Send + Sync + 'static,
    exportar: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let aplicar_busca = move || {
        busca.set(busca_input.get());
        resetar();
    };
    view! {
        <div class="estoque-filtros__linha">
            <form
                class="estoque-filtros__busca"
                on:submit=move |ev| {
                    ev.prevent_default();
                    aplicar_busca();
                }
            >
                <span class="campo-icone campo-icone--cresce">
                    <span class="campo-icone__icone" aria-hidden="true">
                        <Icone arquivo="busca.svg" />
                    </span>
                    <input
                        class="input input--compacto input--com-icone"
                        placeholder="Buscar item, código, SKU, cor…"
                        prop:value=move || busca_input.get()
                        on:input=move |ev| busca_input.set(event_target_value(&ev))
                    />
                </span>
                <button type="submit" class="btn btn--escuro btn--sm">
                    "Buscar"
                </button>
            </form>

            <span class="toolbar-sep"></span>

            <Seletor
                icone="ordenar.svg"
                rotulo="Ordenar"
                opcoes=vec![
                    ("sugerida_desc", "Sugestão"),
                    ("cobertura_asc", "Cobertura +"),
                    ("cobertura_desc", "Cobertura -"),
                    ("disponivel_desc", "Disponível +"),
                    ("disponivel_asc", "Disponível -"),
                    ("recomendada_desc", "Recomendada"),
                    ("produto_asc", "Ordenado A - Z"),
                    ("produto_desc", "Ordenado Z - A"),
                    ("classe_asc", "Classe"),
                ]
                valor=Signal::derive(move || ordem.get())
                ao_escolher=move |v| {
                    ordem.set(v);
                    resetar();
                }
            />
            <Seletor
                icone="filtro.svg"
                rotulo="Status"
                opcoes=vec![
                    ("", "Todos"),
                    ("critico", "Crítico"),
                    ("sem_estoque", "Sem estoque"),
                    ("estoque_baixo", "Estoque baixo"),
                    ("baixo", "Baixo"),
                    ("adequado", "Adequado"),
                    ("alto", "Alto"),
                    ("excessivo", "Excessivo"),
                    ("sem_historico", "Sem histórico"),
                    ("fora_de_linha", "Fora de Linha"),
                ]
                valor=Signal::derive(move || status.get().unwrap_or_default())
                ao_escolher=move |v: String| {
                    status.set((!v.is_empty()).then_some(v));
                    resetar();
                }
            />

            // Exportação: CSV UTF-8 com BOM, do filtro completo (§12). Só um formato, sem menu.
            <button
                type="button"
                class="btn-icone"
                aria-label="Exportar CSV do filtro completo"
                title="Exportar CSV"
                on:click=move |_| exportar()
            >
                <Icone arquivo="exportar.svg" />
            </button>
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
                        <th>"Cor"</th>
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
    // A cor tem coluna própria: o nome fica só com o produto (não repete a variação).
    let nome = i
        .produto
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| i.codigo_estoque.clone());
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
            <td class="tabela__cor">
                <Cor configuracao=i.configuracao.clone() />
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

/// Célula de cor: o primeiro atributo em destaque e os demais numa segunda linha. O `title` traz
/// a configuração inteira, que na coluna estreita sai com reticências.
#[component]
fn Cor(configuracao: Option<String>) -> impl IntoView {
    let partes = cor_partes(configuracao.as_deref());
    if partes.is_empty() {
        return view! { <span class="texto-suave">"—"</span> }.into_any();
    }
    let completo = configuracao.unwrap_or_default();
    let (_, principal) = partes[0].clone();
    let extras = partes[1..]
        .iter()
        .map(|(rot, val)| format!("{rot}: {val}"))
        .collect::<Vec<_>>()
        .join(" · ");
    view! {
        <span class="cor-celula" title=completo>
            <span class="cor-celula__valor">{principal}</span>
            {(!extras.is_empty()).then(|| view! { <span class="cor-celula__extra">{extras}</span> })}
        </span>
    }
    .into_any()
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
                            "Mostrando {} á {} de {} itens",
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
