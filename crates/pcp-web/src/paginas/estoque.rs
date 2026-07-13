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
use crate::componentes::{EstadoVazio, Icone, PaginacaoBotoes, Seletor};
use crate::contexto::{FiltroEstoque, ProdutoSelecionado, SelecaoProducao, Sessao};
use crate::download;
use crate::erro::mensagem_usuario;
use crate::formato::{
    badge_classe, cor_partes, fmt_cobertura, fmt_milhar, nome_classe, nome_exibicao, rotulo_status,
};

#[component]
#[allow(clippy::too_many_lines)] // a maior parte é markup declarativo (view!), não lógica
pub fn PaginaEstoque() -> impl IntoView {
    let sessao = expect_context::<Sessao>();

    // Filtros e paginação (estado do cliente; a consulta vai inteira para o servidor).
    // Busca, filtros, ordenação e página vêm do CONTEXTO: entrar num produto e voltar não pode
    // perder o que o usuário tinha pesquisado nem a página em que estava.
    let FiltroEstoque {
        classe,
        status,
        busca,
        busca_input,
        ordem,
        producao,
        limite,
        deslocamento,
    } = expect_context::<FiltroEstoque>();
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
        producao: producao.get(),
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
                <Filtros status busca busca_input resetar exportar />
                // As contagens vêm da MESMA consulta da lista: mudam com a busca e o status,
                // e nunca dessincronizam do que a tabela mostra.
                <Suspense fallback=|| ()>
                    {move || {
                        dados.get().map(|res| match res {
                            Ok(pag) => {
                                view! {
                                    <div class="estoque-filtros__linha2">
                                        {abas_classe(&pag.contagem_classes, classe, resetar)}
                                        <FiltrosProducao producao resetar />
                                    </div>
                                }
                                    .into_any()
                            }
                            Err(_) => ().into_any(),
                        })
                    }}
                </Suspense>
            </div>

            // A mesma faixa "Mostrando X à Y" do rodapé, logo abaixo do card de filtros.
            <Suspense fallback=|| ()>
                {move || {
                    dados.get().map(|res| match res {
                        Ok(pag) => {
                            let total = pag.total;
                            view! {
                                <div class="lista-info">
                                    <PaginacaoInfo limite deslocamento total />
                                    <PaginacaoBotoes limite deslocamento total />
                                </div>
                            }
                                .into_any()
                        }
                        Err(_) => ().into_any(),
                    })
                }}
            </Suspense>

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
                                    <div class="cartao lista-cartao">
                                        <div class="tabela-cartao">
                                            <Tabela itens=pag.itens ordem resetar />
                                            <Paginacao limite deslocamento total />
                                        </div>
                                    </div>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
            <BotaoOrdens />
        </section>
    }
}

/// Botão flutuante que aparece assim que o primeiro produto é marcado. Leva à criação das Ordens
/// de Produção (que são Solicitações — doc 02 §7.2). Fica no canto da lista, sobre o conteúdo.
#[component]
fn BotaoOrdens() -> impl IntoView {
    let selecao = expect_context::<SelecaoProducao>();
    let n = move || selecao.0.read().len();
    // Sem `>` no atributo: dentro de `view!` o sinal de maior fecharia a tag.
    let tem_selecao = move || !selecao.0.read().is_empty();
    view! {
        <Show when=tem_selecao fallback=|| ()>
            <A href="/producao" attr:class="fab-ordens">
                <Icone arquivo="separacao.svg" />
                <span>
                    {move || {
                        format!(
                            "Produzir {} produto(s)",
                            fmt_milhar(i64::try_from(n()).unwrap_or(0)),
                        )
                    }}
                </span>
            </A>
        </Show>
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
    // Produtos em estado crítico (status canônico do motor — doc 02 §5.2). Substitui a "cobertura
    // média", que era uma média aritmética dominada por produtos mortos com estoque encalhado
    // (alguns com centenas de anos de cobertura) e não descrevia nenhum produto real.
    let criticos: i64 = p
        .por_status
        .iter()
        .filter(|c| c.rotulo == "critico")
        .map(|c| c.quantidade)
        .sum();
    // Fora de linha pela CLASSE (F), não pelo status: é o mesmo universo da aba de filtro. O
    // status `fora_de_linha` é mais restrito e mostraria um número que não bate com a aba.
    let fora_de_linha: i64 = p
        .por_classe
        .iter()
        .filter(|c| c.rotulo == "F")
        .map(|c| c.quantidade)
        .sum();
    view! {
        <div class="kpis">
            <KpiEstoque
                icone="products.svg"
                valor=fmt_milhar(p.total_produtos)
                rotulo="Produtos cadastrados"
            />
            <KpiEstoque
                icone="alerta.svg"
                valor=fmt_milhar(abaixo)
                rotulo="Abaixo do recomendado"
            />
            <KpiEstoque
                icone="nao-conformidade.svg"
                valor=fmt_milhar(criticos)
                rotulo="Produtos críticos"
            />
            <KpiEstoque
                icone="parada.svg"
                valor=fmt_milhar(fora_de_linha)
                rotulo="Produtos fora de linha"
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
/// Três filtros por estado de produção, à direita das abas de classe. Clicar de novo no ativo
/// limpa o filtro. Cores iguais às da linha da lista e do botão do detalhe.
#[component]
fn FiltrosProducao(
    producao: RwSignal<Option<String>>,
    resetar: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    // Ordem cronológica do ciclo de produção: fila -> produzindo -> pronto.
    const ESTADOS: [(&str, &str); 3] = [
        ("aguardando", "Vai Produzir"),
        ("em_producao", "Em Produção"),
        ("recem_produzido", "Produzido"),
    ];
    view! {
        <div class="filtros-prod">
            {ESTADOS
                .into_iter()
                .map(|(chave, rotulo)| {
                    let ativo = move || producao.get().as_deref() == Some(chave);
                    view! {
                        <button
                            type="button"
                            class=format!("filtro-prod filtro-prod--{chave}")
                            class:filtro-prod--ativo=ativo
                            aria-pressed=move || if ativo() { "true" } else { "false" }
                            on:click=move |_| {
                                // Clicar no que já está ativo limpa o filtro.
                                producao.set(if ativo() { None } else { Some(chave.to_owned()) });
                                resetar();
                            }
                        >
                            <span class="filtro-prod__ponto"></span>
                            {rotulo}
                        </button>
                    }
                })
                .collect_view()}
        </div>
    }
}

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
                        <AbaClasse
                            classe
                            rotulo=nome_classe(&cod).to_owned()
                            valor=Some(cod)
                            contagem=qtd
                            resetar
                        />
                    }
                })
                .collect_view()}
        </div>
    }
}

/// Ordem de exibição das abas: primeiro a curva (A→C), depois os estados, com Novo à frente.
fn ordem_classe(c: &str) -> u8 {
    match c {
        "A" => 0,
        "B" => 1,
        "C" => 2,
        "N" => 3,
        "D" => 4,
        "F" => 5,
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
    resetar: impl Fn() + Copy + Send + Sync + 'static,
    exportar: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let aplicar_busca = move || {
        busca.set(busca_input.get());
        resetar();
    };
    view! {
        <div class="estoque-filtros__linha">
            // Sem botão: Enter aplica a busca (o submit do formulário).
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
                        placeholder="Buscar Produto, Código, Cor ou SKU"
                        prop:value=move || busca_input.get()
                        on:input=move |ev| busca_input.set(event_target_value(&ev))
                    />
                </span>
            </form>

            <Seletor
                icone="filtro.svg"
                rotulo="Status"
                opcoes=vec![
                    ("", "Todos"),
                    ("sem_estoque", "Sem estoque"),
                    ("critico", "Crítico"),
                    ("estoque_baixo", "Baixo"),
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

/// Cabeçalho ordenável. O primeiro clique ordena ascendente; o clique seguinte inverte. Só a
/// coluna ativa mostra a seta — as demais ficam limpas, para não poluir o cabeçalho.
#[component]
fn Th(
    rotulo: &'static str,
    /// Prefixo da chave de ordenação da API (`{chave}_asc` / `{chave}_desc`).
    chave: &'static str,
    #[prop(optional)] classe: &'static str,
    ordem: RwSignal<String>,
    resetar: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let asc = move || ordem.get() == format!("{chave}_asc");
    let desc = move || ordem.get() == format!("{chave}_desc");
    let ativa = move || asc() || desc();
    let alternar = move |_| {
        ordem.set(if asc() {
            format!("{chave}_desc")
        } else {
            format!("{chave}_asc")
        });
        resetar();
    };
    view! {
        <th class=classe aria-sort=move || if asc() { "ascending" } else if desc() { "descending" } else { "none" }>
            <button type="button" class="th-ordena" class:th-ordena--ativa=ativa on:click=alternar>
                <span>{rotulo}</span>
                <span class="th-ordena__seta" class:th-ordena__seta--asc=asc>
                    <Show when=ativa fallback=|| ()>
                        <Icone arquivo="seta-baixo.svg" />
                    </Show>
                </span>
            </button>
        </th>
    }
}

#[component]
fn Tabela(
    itens: Vec<LinhaEstoque>,
    ordem: RwSignal<String>,
    resetar: impl Fn() + Copy + Send + Sync + 'static,
) -> impl IntoView {
    view! {
        <div class="tabela-rolavel">
            <table class="tabela tabela--centro">
                <thead>
                    <tr>
                        <th class="tabela__sel" aria-label="Selecionar"></th>
                        <Th rotulo="Código" chave="codigo" ordem resetar />
                        <Th rotulo="Produto" chave="produto" ordem resetar />
                        <Th rotulo="Cor" chave="cor" ordem resetar />
                        <Th rotulo="Classe" chave="classe" ordem resetar />
                        <Th
                            rotulo="Cobertura"
                            chave="cobertura"
                            classe="tabela__nivel-col"
                            ordem
                            resetar
                        />
                        <Th
                            rotulo="Disponível"
                            chave="disponivel"
                            classe="tabela__num"
                            ordem
                            resetar
                        />
                        <Th
                            rotulo="Recomendação"
                            chave="sugerida"
                            classe="tabela__num"
                            ordem
                            resetar
                        />
                        <Th rotulo="Status" chave="status" ordem resetar />
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
    let selecao = expect_context::<SelecaoProducao>();
    let cod_sel = i.codigo_estoque.clone();
    // O que a tela de ordens precisa saber deste produto — sem rebuscar a API depois.
    let item = ProdutoSelecionado {
        codigo: i.codigo_estoque.clone(),
        nome: nome_exibicao(
            i.produto.as_deref(),
            i.configuracao.as_deref(),
            &i.codigo_estoque,
        ),
        qtd_sugerida: i.qtd_sugerida,
    };
    // Estado de produção do item (do One): colore a borda do card-linha. A ordem de produção não
    // tem cor, então o estado vale para todas as cores do mesmo copo.
    let classe_linha = i
        .estado_producao
        .as_deref()
        .map_or_else(String::new, |e| format!("linha-prod linha-prod--{e}"));
    // A cor tem coluna própria: o nome fica só com o produto (não repete a variação).
    let nome = i
        .produto
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| i.codigo_estoque.clone());
    let href = format!("/estoque/{}", i.codigo_estoque);
    // A, B e C cabem num círculo; D, F e N viram pílula, porque carregam o nome inteiro.
    let (classe_abc, rotulo_classe) = badge_classe(&i.classe);
    let cor_st = cor_status(&i.status);
    // Sem recomendação (histórico insuficiente), o motor não tem alvo: não há nível a medir nem
    // quantidade a produzir. Uma barra vazia contra alvo zero seria uma leitura inventada (§3).
    let recomendada = i.estoque_total_recomendado;
    let sem_alvo = recomendada <= 0;
    // Barra de nível: preenchimento = disponível / recomendado (0–100%), cor pelo status.
    // O alvo (recomendado) vai no rótulo; nada é recalculado aqui — só visualização (§3).
    let alvo = recomendada.max(1);
    let pct = ((i.qtd_disponivel as f64 / alvo as f64) * 100.0).clamp(0.0, 100.0);
    let estilo_barra = format!("width:{pct:.0}%;background:{cor_st}");
    view! {
        <tr class=classe_linha>
            <td class="tabela__sel">
                <input
                    class="check-prod"
                    type="checkbox"
                    aria-label=format!("Selecionar {}", i.codigo_estoque)
                    prop:checked=move || selecao.tem(&cod_sel)
                    on:change=move |_| selecao.alternar(item.clone())
                />
            </td>
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
                <span class=classe_abc>{rotulo_classe}</span>
            </td>
            <td class="tabela__nivel-col">
                {if sem_alvo {
                    view! { <SemAlvo /> }.into_any()
                } else {
                    view! {
                        <div class="nivel">
                            <div class="nivel__trilho">
                                <span class="nivel__preenche" style=estilo_barra></span>
                            </div>
                            <span class="nivel__ref">
                                {if i.cobertura_dias >= 999.0 {
                                    "Sem histórico".to_owned()
                                } else {
                                    format!("{} dias", fmt_cobertura(i.cobertura_dias))
                                }}
                            </span>
                        </div>
                    }
                        .into_any()
                }}
            </td>
            <td class="tabela__num">
                <div class="tabela__disp">
                    <span class="tabela__disp-valor">{format!("{} un", fmt_milhar(i.qtd_disponivel))}</span>
                    <span class="tabela__disp-cob">
                        {if sem_alvo {
                            "sem recomendação".to_owned()
                        } else {
                            format!("rec. {} un", fmt_milhar(recomendada))
                        }}
                    </span>
                </div>
            </td>
            <td class="tabela__num tabela__produzir">
                {if sem_alvo || i.qtd_sugerida <= 0 {
                    view! { <SemAlvo /> }.into_any()
                } else {
                    view! { {format!("{} un", fmt_milhar(i.qtd_sugerida))} }.into_any()
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

/// Marca de célula sem valor a exibir. O `title` diz por que está vazia — o asterisco sozinho não
/// explica nada a quem chega na tela.
#[component]
fn SemAlvo() -> impl IntoView {
    view! {
        <span class="sem-alvo" title="Sem histórico suficiente para recomendar">
            "*"
        </span>
    }
}

/// Célula de cor: o primeiro atributo em destaque e os demais numa segunda linha. Nada é cortado —
/// a cor identifica o produto tanto quanto o nome, então a célula cresce em altura se preciso.
#[component]
fn Cor(configuracao: Option<String>) -> impl IntoView {
    let partes = cor_partes(configuracao.as_deref());
    if partes.is_empty() {
        return view! { <span class="texto-suave">"—"</span> }.into_any();
    }
    let (_, principal) = partes[0].clone();
    let extras = partes[1..]
        .iter()
        .map(|(rot, val)| format!("{rot}: {val}"))
        .collect::<Vec<_>>()
        .join(" · ");
    view! {
        <span class="cor-celula">
            <span class="cor-celula__valor">{principal}</span>
            {(!extras.is_empty()).then(|| view! { <span class="cor-celula__extra">{extras}</span> })}
        </span>
    }
    .into_any()
}

#[component]
fn Paginacao(limite: RwSignal<i64>, deslocamento: RwSignal<i64>, total: i64) -> impl IntoView {
    view! {
        <nav class="paginacao">
            <PaginacaoInfo limite deslocamento total />
            <PaginacaoBotoes limite deslocamento total />
        </nav>
    }
}

/// Texto "Mostrando X à Y de Z itens" (a faixa da página atual). Aparece abaixo do card de
/// filtros e no rodapé da tabela; lê os mesmos sinais, então os dois dizem sempre o mesmo.
#[component]
fn PaginacaoInfo(limite: RwSignal<i64>, deslocamento: RwSignal<i64>, total: i64) -> impl IntoView {
    let inicio = move || {
        if total == 0 {
            0
        } else {
            deslocamento.get() + 1
        }
    };
    let fim = move || (deslocamento.get() + limite.get()).min(total);
    view! {
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
    }
}
