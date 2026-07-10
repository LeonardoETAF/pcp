//! Detalhe do Produto (doc 03 §4): cabeçalho, métricas e — nas próximas seções — status/histórico
//! de produção e movimentação. Frontend burro (§3): tudo vem pronto da API.

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::api::{
    produto_atividade, produto_detalhe, produto_insights, DetalheProduto, Insights,
    MetricasProduto, Movimento, OrdemProducao, StatusProducao, VendaMesProduto,
};
use crate::componentes::Icone;
use crate::contexto::Sessao;
use crate::erro::mensagem_usuario;
use crate::formato::{fmt_cobertura, fmt_milhar, nome_exibicao};

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
                            Ok(Some(d)) => corpo(d).into_any(),
                        })
                }}
            </Suspense>
        </section>
    }
}

/// Conteúdo completo do detalhe: cabeçalho, resumo, cards, gráfico, tendências, métricas e as
/// seções de produção/movimentação.
// `d` chega por valor (é a linha carregada, consumida na montagem da view).
#[allow(clippy::too_many_lines, clippy::needless_pass_by_value)]
fn corpo(d: DetalheProduto) -> impl IntoView {
    let nome = nome_exibicao(
        d.produto.as_deref(),
        d.configuracao.as_deref(),
        &d.codigo_estoque,
    );
    let classe = d.classe.clone();
    let codigo = d.codigo_estoque.clone();
    let sku = d.sku.clone().filter(|s| !s.is_empty());

    // A atividade e os insights são carregados UMA vez cada. Insights são estatísticos (pcp-ai),
    // calculados no backend — sem custo de IA/LLM.
    let sessao = expect_context::<Sessao>();
    let cod = d.codigo_estoque.clone();
    let ativ = Resource::new(
        move || (sessao.0.get(), cod.clone()),
        |(token, codigo)| async move {
            match token {
                Some(t) => produto_atividade(t, codigo).await.ok(),
                None => None,
            }
        },
    );
    let cod2 = d.codigo_estoque.clone();
    let insights = Resource::new(
        move || (sessao.0.get(), cod2.clone()),
        |(token, codigo)| async move {
            match token {
                Some(t) => produto_insights(t, codigo).await.ok(),
                None => None,
            }
        },
    );

    let m = d.metricas.clone();

    view! {
        <header class="prod-cab">
            <A href="/estoque" attr:class="icone-btn-claro" attr:aria-label="Voltar ao estoque" attr:title="Voltar">
                <Icone arquivo="seta-esquerda.svg" />
            </A>
            <div class="prod-cab__id">
                <h1 class="pagina__titulo">{nome}</h1>
                <p class="prod-cab__sub">
                    {format!("Código: {codigo}")}
                    {sku.map(|s| format!(" · SKU: {s}"))}
                </p>
            </div>
        </header>

        <Suspense fallback=|| view! { <div class="cards-resumo cards-resumo--sk"></div> }>
            {
                let m = m.clone();
                let classe = classe.clone();
                move || {
                    let ins = insights.get().flatten();
                    cards_resumo(&m, &classe, ins.as_ref()).into_any()
                }
            }
        </Suspense>

        <Suspense fallback=|| ()>
            {move || {
                ativ.get().flatten().map(|a| {
                    view! { <GraficoVendasAnual meses=a.vendas_mensais /> }
                })
            }}
        </Suspense>

        <section class="cartao prod-status-secao">
            <Suspense fallback=|| ()>
                {move || {
                    ativ.get().flatten().map(|a| {
                        view! { <BotaoStatusProducao s=a.status_producao /> }
                    })
                }}
            </Suspense>
        </section>

        <Suspense fallback=|| {
            view! { <p class="texto-suave">"Carregando atividade…"</p> }
        }>
            {move || {
                ativ.get().flatten().map_or_else(
                    || ().into_any(),
                    |a| {
                        view! {
                            {historico_producao(&a.producao)}
                            {historico_movimentacao(&a.movimentos)}
                        }
                            .into_any()
                    },
                )
            }}
        </Suspense>
    }
}

/// Quatro cards de resumo (estilo do mockup): estoque, performance, recomendação e alertas.
/// `insights` pode faltar (falha estatística) — os campos de IA caem para "—".
fn cards_resumo(m: &MetricasProduto, classe: &str, insights: Option<&Insights>) -> impl IntoView {
    #[allow(clippy::cast_precision_loss)] // quantidades de estoque cabem exatas em f64
    let pct_disp = if m.qtd_estoque > 0 {
        (m.qtd_disponivel as f64 / m.qtd_estoque as f64 * 100.0).round()
    } else {
        0.0
    };
    let sazonal = insights.map_or_else(
        || "—".to_owned(),
        |i| format!("{:.0}%", i.forca_sazonal * 100.0),
    );
    let confianca = insights.map_or_else(
        || "—".to_owned(),
        |i| format!("{:.0}%", i.confianca * 100.0),
    );
    view! {
        <div class="cards-resumo">
            <article class="card-resumo">
                <header class="card-resumo__cab">
                    <span class="card-resumo__titulo">"Estoque atual"</span>
                    <Icone arquivo="inventory.svg" />
                </header>
                <span class="card-resumo__valor">{fmt_milhar(m.qtd_estoque)}</span>
                <span class="card-resumo__sub">
                    {format!("Disponível: {} ({:.0}%)", fmt_milhar(m.qtd_disponivel), pct_disp)}
                </span>
            </article>

            <article class="card-resumo">
                <header class="card-resumo__cab">
                    <span class="card-resumo__titulo">"Performance"</span>
                    <span class="card-resumo__chip">{format!("{}/dia", fmt_media(m.media_diaria))}</span>
                </header>
                <span class="card-resumo__valor">{format!("Classe {classe}")}</span>
                <span class="card-resumo__sub">
                    {format!("Volume Anual: {}", fmt_milhar(m.volume_janela))}
                </span>
            </article>

            <article class="card-resumo">
                <header class="card-resumo__cab">
                    <span class="card-resumo__titulo">"Recomendação"</span>
                    <span class="card-resumo__chip">{format!("{sazonal} sazonal")}</span>
                </header>
                <span class="card-resumo__valor">{fmt_milhar(m.qtd_sugerida)}</span>
                <span class="card-resumo__sub">
                    {format!("Recomendada: {} un", fmt_milhar(m.estoque_total_recomendado))}
                </span>
                <span class="card-resumo__nota">{format!("Confiança {confianca}")}</span>
            </article>

            <article class="card-resumo">
                <header class="card-resumo__cab">
                    <span class="card-resumo__titulo">"Cobertura"</span>
                    <Icone arquivo="relogio.svg" />
                </header>
                <span class="card-resumo__valor">{cobertura_valor(m.cobertura_dias)}</span>
                <span class="card-resumo__sub">
                    {format!("Recomendada: {} un", fmt_milhar(m.estoque_total_recomendado))}
                </span>
            </article>
        </div>
    }
}

/// Média diária arredondada, com separador de milhar (ex.: 2424.7 → "2.425").
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // média pequena e não-negativa
fn fmt_media(v: f64) -> String {
    fmt_milhar(v.max(0.0).round() as i64)
}

/// Valor de cobertura para o card: dias com uma casa, ou "Sem histórico" (sentinela 999).
fn cobertura_valor(dias: f64) -> String {
    if dias >= 999.0 {
        "Sem histórico".to_owned()
    } else {
        format!("{} dias", fmt_cobertura(dias))
    }
}

/// Abreviação de mês (1–12) para o eixo do gráfico anual.
fn mes_abrev(m: i32) -> &'static str {
    const M: [&str; 12] = [
        "Jan", "Fev", "Mar", "Abr", "Mai", "Jun", "Jul", "Ago", "Set", "Out", "Nov", "Dez",
    ];
    usize::try_from(m - 1)
        .ok()
        .and_then(|i| M.get(i).copied())
        .unwrap_or("")
}

/// Gráfico anual comparativo: venda por mês do ano corrente vs. o anterior. Cada mês tem duas
/// barras CENTRADAS no mesmo eixo — a do ano anterior mais larga (ao fundo) e a do ano atual mais
/// estreita (à frente), uma dentro da outra.
#[component]
#[allow(clippy::cast_precision_loss)] // 12 meses; quantidades cabem exatas em f64
fn GraficoVendasAnual(meses: Vec<VendaMesProduto>) -> impl IntoView {
    const W: f64 = 720.0;
    const H: f64 = 200.0;
    if meses.is_empty() {
        return view! {
            <section class="cartao">
                <header class="cartao__cab">
                    <h2 class="cartao__titulo">"Vendas Anual"</h2>
                </header>
                <p class="estado-vazio">"Sem vendas registradas."</p>
            </section>
        }
        .into_any();
    }
    // Dois anos mais recentes presentes: `anterior` e `atual`.
    let mut anos: Vec<i32> = meses.iter().map(|v| v.ano).collect();
    anos.sort_unstable();
    anos.dedup();
    let atual = anos.last().copied().unwrap_or_default();
    let anterior = atual - 1;
    // Vetores por mês (1–12).
    let serie = |ano: i32| -> [i64; 12] {
        let mut v = [0_i64; 12];
        for x in meses.iter().filter(|x| x.ano == ano) {
            if let Some(slot) = usize::try_from(x.mes - 1).ok().filter(|&i| i < 12) {
                v[slot] = x.total;
            }
        }
        v
    };
    let va = serie(atual);
    let vp = serie(anterior);
    let max = va
        .iter()
        .chain(vp.iter())
        .copied()
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    let slot = W / 12.0;
    let barras = (0..12)
        .flat_map(|m| {
            let centro = (m as f64 + 0.5) * slot;
            let alt = |q: i64| (q as f64 / max) * H;
            // Anterior: larga, ao fundo. Atual: estreita, à frente. Ambas centradas em `centro`.
            let l_ant = slot * 0.62;
            let l_atu = slot * 0.34;
            let ha = alt(va[m]);
            let hp = alt(vp[m]);
            [
                view! {
                    <rect
                        x=format!("{:.2}", centro - l_ant / 2.0)
                        y=format!("{:.2}", H - hp)
                        width=format!("{l_ant:.2}")
                        height=format!("{hp:.2}")
                        class="graf__barra graf__barra--anterior"
                    />
                },
                view! {
                    <rect
                        x=format!("{:.2}", centro - l_atu / 2.0)
                        y=format!("{:.2}", H - ha)
                        width=format!("{l_atu:.2}")
                        height=format!("{ha:.2}")
                        class="graf__barra"
                    />
                },
            ]
        })
        .collect_view();
    let rotulos = (0..12_i32)
        .map(|m| {
            let x = (f64::from(m) + 0.5) * slot;
            view! {
                <text x=format!("{x:.1}") y="196" class="graf__rotulo-mes">
                    {mes_abrev(m + 1)}
                </text>
            }
        })
        .collect_view();
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Vendas Anual"</h2>
                <div class="graf__legenda">
                    <span class="graf__leg graf__leg--anterior">{anterior.to_string()}</span>
                    <span class="graf__leg graf__leg--atual">{atual.to_string()}</span>
                </div>
            </header>
            <svg class="grafico grafico--anual" viewBox=format!("0 0 {W} {} ", H + 16.0)>
                {barras}
                {rotulos}
            </svg>
        </section>
    }
    .into_any()
}

/// Status de produção como botão informativo./// Status de produção como botão informativo. Só ABRE (mostra produzido × falta) se houver ordem
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
        <div class="prod-status">
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
        </div>
    }
}

/// Histórico de produção: ordens da linha em grade de cards (mais recentes primeiro).
fn historico_producao(ordens: &[OrdemProducao]) -> impl IntoView {
    let cards = ordens
        .iter()
        .map(|o| {
            let data = o.data.clone().unwrap_or_else(|| "—".to_owned());
            let lote = o.lote.map_or_else(|| "—".to_owned(), |l| l.to_string());
            let status = o.status.clone().unwrap_or_default();
            let classe_st = format!("badge badge--producao-{}", status.to_lowercase());
            view! {
                <div class="ativ-card">
                    <div class="ativ-card__topo">
                        <span class="ativ-card__data">{fmt_data(&data)}</span>
                        <span class=classe_st>{rotulo_producao(&status)}</span>
                    </div>
                    <div class="ativ-card__linha">
                        <span class="ativ-card__rotulo">"Quantidade"</span>
                        <span class="ativ-card__valor">{format!("{} un", fmt_milhar(o.quantidade))}</span>
                    </div>
                    <div class="ativ-card__linha">
                        <span class="ativ-card__rotulo">"Lote"</span>
                        <span class="ativ-card__valor">{lote}</span>
                    </div>
                </div>
            }
        })
        .collect_view();
    view! {
        <section class="prod-secao">
            <h2 class="prod-secao__titulo">"Histórico de produção"</h2>
            {if ordens.is_empty() {
                view! { <p class="estado-vazio">"Sem ordens de produção registradas."</p> }
                    .into_any()
            } else {
                view! { <div class="ativ-grid">{cards}</div> }.into_any()
            }}
        </section>
    }
}

/// Histórico de movimentação: kardex da linha em grade de cards (entradas/saídas, recentes primeiro).
fn historico_movimentacao(movs: &[Movimento]) -> impl IntoView {
    let cards = movs
        .iter()
        .map(|m| {
            let entrada = m.quantidade >= 0;
            let classe_qtd = if entrada {
                "ativ-card__valor mov--entrada"
            } else {
                "ativ-card__valor mov--saida"
            };
            let sinal = if entrada { "+" } else { "" };
            view! {
                <div class="ativ-card">
                    <div class="ativ-card__topo">
                        <span class="ativ-card__data">{fmt_data(&m.data)}</span>
                        <span class="badge badge--mov">{rotulo_movimento(&m.tipo)}</span>
                    </div>
                    <div class="ativ-card__linha">
                        <span class="ativ-card__rotulo">"Quantidade"</span>
                        <span class=classe_qtd>
                            {format!("{sinal}{} un", fmt_milhar(m.quantidade))}
                        </span>
                    </div>
                    <div class="ativ-card__linha">
                        <span class="ativ-card__rotulo">"Saldo"</span>
                        <span class="ativ-card__valor">{format!("{} un", fmt_milhar(m.saldo))}</span>
                    </div>
                </div>
            }
        })
        .collect_view();
    view! {
        <section class="prod-secao">
            <h2 class="prod-secao__titulo">"Histórico de movimentação"</h2>
            {if movs.is_empty() {
                view! { <p class="estado-vazio">"Sem movimentações registradas."</p> }.into_any()
            } else {
                view! { <div class="ativ-grid">{cards}</div> }.into_any()
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
