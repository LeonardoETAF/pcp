//! Detalhe do Produto (doc 03 §4): cabeçalho, métricas e — nas próximas seções — status/histórico
//! de produção e movimentação. Frontend burro (§3): tudo vem pronto da API.

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::api::{
    produto_atividade, produto_detalhe, Atividade, DetalheProduto, MetricasProduto, Movimento,
    OrdemProducao, StatusProducao,
};
use crate::componentes::Icone;
use crate::contexto::Sessao;
use crate::erro::mensagem_usuario;
use crate::formato::{fmt_cobertura, fmt_milhar, nome_exibicao, rotulo_status};

#[component]
pub fn DetalheProdutoPagina() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let params = use_params_map();
    let codigo = move || params.read().get("codigo").unwrap_or_default();
    let tick = RwSignal::new(0_u32);

    let dados = Resource::new(
        move || (sessao.0.get(), codigo(), tick.get()),
        |(token, codigo, _)| async move {
            match token {
                Some(t) => produto_detalhe(t, codigo).await,
                None => Ok(None),
            }
        },
    );

    view! {
        <section class="pagina">
            <Suspense fallback=|| view! { <Esqueleto /> }>
                {move || {
                    dados
                        .get()
                        .map(|res| match res {
                            Err(e) => {
                                view! {
                                    <div class="estado-erro">
                                        <p class="form-auth__erro">{mensagem_usuario(&e)}</p>
                                        <button
                                            class="btn btn--secundario"
                                            on:click=move |_| tick.update(|n| *n += 1)
                                        >
                                            "Tentar novamente"
                                        </button>
                                    </div>
                                }
                                    .into_any()
                            }
                            Ok(None) => {
                                view! {
                                    <div class="estado-vazio">
                                        <p>"Produto não encontrado."</p>
                                        <A href="/estoque" attr:class="btn btn--secundario">
                                            "Voltar ao estoque"
                                        </A>
                                    </div>
                                }
                                    .into_any()
                            }
                            Ok(Some(d)) => corpo(&d).into_any(),
                        })
                }}
            </Suspense>
        </section>
    }
}

/// Conteúdo completo do detalhe: cabeçalho, métricas e as seções de produção/movimentação.
fn corpo(d: &DetalheProduto) -> impl IntoView {
    let nome = nome_exibicao(
        d.produto.as_deref(),
        d.configuracao.as_deref(),
        &d.codigo_estoque,
    );
    let classe_abc = format!("badge badge--abc-{}", d.classe.to_lowercase());
    let classe_status = format!("badge badge--status-{}", d.status);
    let status_rotulo = rotulo_status(&d.status);
    let codigo = d.codigo_estoque.clone();
    let sku = d.sku.clone().filter(|s| !s.is_empty());

    view! {
        <section class="cartao prod-cab">
            <A href="/estoque" attr:class="icone-btn-claro" attr:aria-label="Voltar ao estoque" attr:title="Voltar">
                <Icone arquivo="seta-esquerda.svg" />
            </A>
            <div class="prod-cab__id">
                <h1 class="pagina__titulo">{nome}</h1>
                <div class="prod-cab__meta">
                    <span class="prod-cab__codigo">{codigo}</span>
                    {sku.map(|s| view! { <span class="prod-cab__sku">{s}</span> })}
                    <span class=classe_abc>{d.classe.clone()}</span>
                    <span class=classe_status>{status_rotulo}</span>
                </div>
            </div>
        </section>

        {metricas(&d.metricas)}
        <Atividades codigo=d.codigo_estoque.clone() />
    }
}

/// Carrega e exibe status de produção, histórico de produção e histórico de movimentação.
#[component]
fn Atividades(codigo: String) -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let dados = Resource::new(
        move || (sessao.0.get(), codigo.clone()),
        |(token, codigo)| async move {
            match token {
                Some(t) => produto_atividade(t, codigo).await.ok(),
                None => None,
            }
        },
    );
    view! {
        <Suspense fallback=|| {
            view! { <p class="texto-suave">"Carregando atividade…"</p> }
        }>
            {move || {
                dados.get().flatten().map_or_else(
                    || ().into_any(),
                    |a| view! { {secoes_atividade(&a)} }.into_any(),
                )
            }}
        </Suspense>
    }
}

fn secoes_atividade(a: &Atividade) -> impl IntoView {
    view! {
        <BotaoStatusProducao s=a.status_producao.clone() />
        {historico_producao(&a.producao)}
        {historico_movimentacao(&a.movimentos)}
    }
}

/// Status de produção como botão informativo. Só ABRE (mostra produzido × falta) se houver ordem
/// EM PRODUÇÃO — aguardando/sem ordem não tem progresso a mostrar, então o botão fica inerte.
#[component]
fn BotaoStatusProducao(s: StatusProducao) -> impl IntoView {
    let em_producao = s.em_producao > 0;
    let aberto = RwSignal::new(false);
    let falta = (s.planejado_em_producao - s.produzido_em_producao).max(0);
    let rotulo = if em_producao {
        format!("Em produção — {} ordem(ns)", fmt_milhar(s.em_producao))
    } else if s.aguardando > 0 {
        format!(
            "Aguardando produção — {} ordem(ns)",
            fmt_milhar(s.aguardando)
        )
    } else {
        "Sem produção em andamento".to_owned()
    };
    view! {
        <section class="cartao prod-status">
            <button
                type="button"
                class="prod-status__btn"
                class:prod-status__btn--ativo=move || em_producao
                disabled=(!em_producao).then_some("")
                aria-expanded=move || if aberto.get() { "true" } else { "false" }
                on:click=move |_| {
                    if em_producao {
                        aberto.update(|a| *a = !*a);
                    }
                }
            >
                <span class="prod-status__ponto" class:prod-status__ponto--on=move || em_producao></span>
                <span class="prod-status__rotulo">{rotulo}</span>
                {em_producao
                    .then(|| view! { <span class="prod-status__seta" class:prod-status__seta--aberto=move || aberto.get()><Icone arquivo="seta-baixo.svg" /></span> })}
            </button>
            <Show when=move || aberto.get() && em_producao fallback=|| ()>
                <div class="prod-metricas prod-status__detalhe">
                    {[
                        ("Planejado", format!("{} un", fmt_milhar(s.planejado_em_producao))),
                        ("Já produzido", format!("{} un", fmt_milhar(s.produzido_em_producao))),
                        ("Falta produzir", format!("{} un", fmt_milhar(falta))),
                    ]
                        .into_iter()
                        .map(|(rotulo, valor)| {
                            view! {
                                <div class="metrica-card">
                                    <span class="metrica-card__rotulo">{rotulo}</span>
                                    <span class="metrica-card__valor">{valor}</span>
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            </Show>
        </section>
    }
}

/// Histórico de produção: ordens da linha (mais recentes primeiro).
fn historico_producao(ordens: &[OrdemProducao]) -> impl IntoView {
    let linhas = ordens
        .iter()
        .map(|o| {
            let data = o.data.clone().unwrap_or_else(|| "—".to_owned());
            let lote = o.lote.map_or_else(|| "—".to_owned(), |l| l.to_string());
            let status = o.status.clone().unwrap_or_default();
            let classe_st = format!("badge badge--producao-{}", status.to_lowercase());
            view! {
                <tr>
                    <td class="tabela__cod">{fmt_data(&data)}</td>
                    <td class="tabela__num">{format!("{} un", fmt_milhar(o.quantidade))}</td>
                    <td>
                        <span class=classe_st>{rotulo_producao(&status)}</span>
                    </td>
                    <td class="tabela__cod">{lote}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Histórico de produção"</h2>
            </header>
            {if ordens.is_empty() {
                view! { <p class="texto-suave">"Sem ordens de produção registradas."</p> }
                    .into_any()
            } else {
                view! {
                    <div class="tabela-rolavel">
                        <table class="tabela">
                            <thead>
                                <tr>
                                    <th>"Data"</th>
                                    <th class="tabela__num">"Quantidade"</th>
                                    <th>"Status"</th>
                                    <th>"Lote"</th>
                                </tr>
                            </thead>
                            <tbody>{linhas}</tbody>
                        </table>
                    </div>
                }
                    .into_any()
            }}
        </section>
    }
}

/// Histórico de movimentação: kardex da linha (entradas e saídas, mais recentes primeiro).
fn historico_movimentacao(movs: &[Movimento]) -> impl IntoView {
    let linhas = movs
        .iter()
        .map(|m| {
            let entrada = m.quantidade >= 0;
            let classe_qtd = if entrada {
                "mov--entrada"
            } else {
                "mov--saida"
            };
            let sinal = if entrada { "+" } else { "" };
            view! {
                <tr>
                    <td class="tabela__cod">{fmt_data(&m.data)}</td>
                    <td>
                        <span class="badge badge--mov">{rotulo_movimento(&m.tipo)}</span>
                    </td>
                    <td class=format!("tabela__num {classe_qtd}")>
                        {format!("{sinal}{} un", fmt_milhar(m.quantidade))}
                    </td>
                    <td class="tabela__num">{format!("{} un", fmt_milhar(m.saldo))}</td>
                </tr>
            }
        })
        .collect_view();
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Histórico de movimentação"</h2>
            </header>
            {if movs.is_empty() {
                view! { <p class="texto-suave">"Sem movimentações registradas."</p> }.into_any()
            } else {
                view! {
                    <div class="tabela-rolavel">
                        <table class="tabela">
                            <thead>
                                <tr>
                                    <th>"Data"</th>
                                    <th>"Tipo"</th>
                                    <th class="tabela__num">"Quantidade"</th>
                                    <th class="tabela__num">"Saldo"</th>
                                </tr>
                            </thead>
                            <tbody>{linhas}</tbody>
                        </table>
                    </div>
                }
                    .into_any()
            }}
        </section>
    }
}

/// Rótulo pt-BR do status da ordem de produção.
fn rotulo_producao(s: &str) -> &'static str {
    match s {
        "AGUARDANDO" => "Aguardando",
        "PRODUCAO" => "Em produção",
        "FINALIZADO" => "Finalizado",
        "CANCELADO" => "Cancelado",
        _ => "—",
    }
}

/// Rótulo pt-BR do tipo de movimento do kardex.
fn rotulo_movimento(t: &str) -> &'static str {
    match t {
        "VENDA" => "Venda",
        "DEVOLUCAO_VENDA" => "Devolução",
        "PRODUCAO" => "Produção",
        "SEPARACAO_VENDA" => "Separação (venda)",
        "SEPARACAO_PRODUCAO" => "Separação (produção)",
        "INVENTARIO" => "Inventário",
        "AJUSTE" => "Ajuste",
        "LOCAL_ESTOQUE" => "Transferência",
        "RESERVA_TEMPORARIA" => "Reserva",
        _ => "Movimento",
    }
}

/// Converte "AAAA-MM-DD" em "DD/MM/AAAA" (formato BR, §12); devolve o original se não casar.
fn fmt_data(iso: &str) -> String {
    match iso
        .split_once('-')
        .and_then(|(a, resto)| resto.split_once('-').map(|(m, d)| format!("{d}/{m}/{a}")))
    {
        Some(br) => br,
        None => iso.to_owned(),
    }
}

/// Grade de métricas (doc 03 §4.1).
fn metricas(m: &MetricasProduto) -> impl IntoView {
    let cards = vec![
        ("Estoque total", fmt_milhar(m.qtd_estoque)),
        ("Reserva", fmt_milhar(m.qtd_reserva)),
        ("Disponível", fmt_milhar(m.qtd_disponivel)),
        ("Cobertura (dias)", fmt_cobertura(m.cobertura_dias)),
        ("Demanda média/dia", format!("{:.1}", m.media_diaria)),
        ("Estoque de segurança", fmt_milhar(m.estoque_seguranca)),
        ("Estoque mínimo", fmt_milhar(m.estoque_minimo)),
        ("Recomendada", fmt_milhar(m.estoque_total_recomendado)),
        ("Sugestão de produção", fmt_milhar(m.qtd_sugerida)),
        ("Volume (janela)", fmt_milhar(m.volume_janela)),
        ("Dias com venda", fmt_milhar(m.dias_com_vendas)),
        ("Outliers", fmt_milhar(m.outliers_detectados)),
        ("Coef. de variação", format!("{:.2}", m.coef_variacao)),
    ];
    view! {
        <div class="prod-metricas">
            {cards
                .into_iter()
                .map(|(rotulo, valor)| {
                    view! {
                        <div class="metrica-card">
                            <span class="metrica-card__rotulo">{rotulo}</span>
                            <span class="metrica-card__valor">{valor}</span>
                        </div>
                    }
                })
                .collect_view()}
        </div>
    }
}

#[component]
fn Esqueleto() -> impl IntoView {
    view! {
        <div class="prod-esqueleto">
            <div class="sk sk--barra"></div>
            <div class="sk sk--bloco"></div>
            <div class="sk sk--grade"></div>
        </div>
    }
}
