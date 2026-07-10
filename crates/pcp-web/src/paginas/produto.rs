//! Detalhe do Produto (doc 03 §4): cabeçalho, métricas e — nas próximas seções — status/histórico
//! de produção e movimentação. Frontend burro (§3): tudo vem pronto da API.

use chrono::{Datelike, NaiveDate};
use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::api::{
    produto_atividade, produto_detalhe, produto_insights, DetalheProduto, Insights,
    MetricasProduto, Movimento, OrdemProducao, StatusProducao, VendaMesProduto,
};
use crate::componentes::{Icone, PaginacaoBotoes};
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
            <Suspense fallback=|| ()>
                {move || {
                    ativ.get().flatten().map(|a| {
                        view! { <BotaoStatusProducao s=a.status_producao /> }
                    })
                }}
            </Suspense>
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

        <Suspense fallback=|| {
            view! { <p class="texto-suave">"Carregando atividade…"</p> }
        }>
            {move || {
                ativ.get().flatten().map_or_else(
                    || ().into_any(),
                    |a| {
                        view! {
                            <HistoricoProducao ordens=a.producao />
                            <HistoricoMovimentacao movimentos=a.movimentos />
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
                <span class="card-resumo__nota">{format!("Confiança {confianca}")}</span>
            </article>

            <article class="card-resumo">
                <header class="card-resumo__cab">
                    <span class="card-resumo__titulo">"Cobertura"</span>
                    <Icone arquivo="relogio.svg" />
                </header>
                <span class="card-resumo__valor">{cobertura_valor(m.cobertura_dias)}</span>
                <span class="card-resumo__sub">
                    {format!("Recomendado: {} un", fmt_milhar(m.estoque_total_recomendado))}
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
            // Anterior: larga, ao fundo. Atual: mais estreita, à frente. Ocupam bem a fatia.
            let l_ant = slot * 0.82;
            let l_atu = slot * 0.5;
            let ha = alt(va[m]);
            let hp = alt(vp[m]);
            [
                view! {
                    <rect
                        x=format!("{:.2}", centro - l_ant / 2.0)
                        y=format!("{:.2}", H - hp)
                        width=format!("{l_ant:.2}")
                        height=format!("{hp:.2}")
                        rx="4"
                        class="graf__barra graf__barra--anterior"
                    />
                },
                view! {
                    <rect
                        x=format!("{:.2}", centro - l_atu / 2.0)
                        y=format!("{:.2}", H - ha)
                        width=format!("{l_atu:.2}")
                        height=format!("{ha:.2}")
                        rx="4"
                        class="graf__barra"
                    />
                },
            ]
        })
        .collect_view();
    // Zona de hover por mês (HTML sobre o SVG): mostra os dois valores daquele mês. Serve também
    // como célula do rótulo, garantindo o alinhamento com as fatias.
    let hover = RwSignal::new(None::<usize>);
    let colunas = (0..12)
        .map(|m| {
            let atu = va[m];
            let ant = vp[m];
            view! {
                <div
                    class="graf__col"
                    class:graf__col--hover=move || hover.get() == Some(m)
                    on:mouseenter=move |_| hover.set(Some(m))
                    on:mouseleave=move |_| hover.set(None)
                >
                    <Show when=move || hover.get() == Some(m) fallback=|| ()>
                        <div class="graf__tip">
                            <span class="graf__tip-mes">{mes_abrev(i32::try_from(m + 1).unwrap_or(1))}</span>
                            <span class="graf__tip-linha">
                                <span class="graf__tip-cor graf__tip-cor--ant"></span>
                                {format!("{anterior}: {} un", fmt_milhar(ant))}
                            </span>
                            <span class="graf__tip-linha">
                                <span class="graf__tip-cor graf__tip-cor--atu"></span>
                                {format!("{atual}: {} un", fmt_milhar(atu))}
                            </span>
                        </div>
                    </Show>
                    <span class="graf__mes">{mes_abrev(i32::try_from(m + 1).unwrap_or(1))}</span>
                </div>
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
            <div class="grafico-anual">
                <div class="grafico-anual__area">
                    <svg
                        class="grafico grafico--anual"
                        viewBox=format!("0 0 {W} {H}")
                        preserveAspectRatio="none"
                    >
                        {barras}
                    </svg>
                    <div class="graf__cols">{colunas}</div>
                </div>
            </div>
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

/// Agrupa itens já ordenados por data (desc) em `(data, itens)` — datas consecutivas caem no mesmo
/// grupo. `chave` extrai a data de exibição de cada item.
fn agrupar_por_data<T, F: Fn(&T) -> String>(itens: &[T], chave: F) -> Vec<(String, Vec<&T>)> {
    let mut grupos: Vec<(String, Vec<&T>)> = Vec::new();
    for it in itens {
        let d = chave(it);
        match grupos.last_mut() {
            Some((data, v)) if *data == d => v.push(it),
            _ => grupos.push((d, vec![it])),
        }
    }
    grupos
}

/// Quantas datas por página nos históricos.
const DATAS_POR_PAGINA: i64 = 15;

/// Uma fatia paginada de grupos-por-data já filtrada.
struct PaginaDatas<'a, T> {
    grupos: Vec<(String, Vec<&'a T>)>,
    total: i64,
}

/// Agrupa por data, aplica a janela [`inicio`, `fim`] (ISO, comparação lexicográfica = cronológica)
/// e recorta a página corrente. Limites vazios não restringem aquele lado.
fn pagina_datas<'a, T, F: Fn(&T) -> String>(
    itens: &'a [T],
    chave: F,
    inicio: &str,
    fim: &str,
    desloc: i64,
) -> PaginaDatas<'a, T> {
    let mut grupos = agrupar_por_data(itens, chave);
    if !inicio.is_empty() {
        grupos.retain(|(d, _)| d.as_str() >= inicio);
    }
    if !fim.is_empty() {
        grupos.retain(|(d, _)| d.as_str() <= fim);
    }
    let total = i64::try_from(grupos.len()).unwrap_or(i64::MAX);
    let ini = usize::try_from(desloc).unwrap_or(0);
    let pag = usize::try_from(DATAS_POR_PAGINA).unwrap_or(15);
    let grupos = grupos.into_iter().skip(ini).take(pag).collect();
    PaginaDatas { grupos, total }
}

/// Cabeçalho de um histórico: título + filtro por janela de tempo.
#[component]
fn HistoricoCab(
    titulo: &'static str,
    inicio: RwSignal<String>,
    fim: RwSignal<String>,
    ref_data: String,
) -> impl IntoView {
    view! {
        <header class="prod-secao__cab">
            <h2 class="prod-secao__titulo">{titulo}</h2>
            <FiltroData inicio fim ref_data />
        </header>
    }
}

/// Data ISO ("AAAA-MM-DD") → `NaiveDate`.
fn parse_iso(s: &str) -> Option<NaiveDate> {
    let mut partes = s.split('-');
    let ano: i32 = partes.next()?.parse().ok()?;
    let mes: u32 = partes.next()?.parse().ok()?;
    let dia: u32 = partes.next()?.parse().ok()?;
    NaiveDate::from_ymd_opt(ano, mes, dia)
}

/// `NaiveDate` → ISO ("AAAA-MM-DD").
fn iso(d: NaiveDate) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

/// Nome do mês em pt-BR.
fn nome_mes(m: u32) -> &'static str {
    const M: [&str; 12] = [
        "Janeiro",
        "Fevereiro",
        "Março",
        "Abril",
        "Maio",
        "Junho",
        "Julho",
        "Agosto",
        "Setembro",
        "Outubro",
        "Novembro",
        "Dezembro",
    ];
    usize::try_from(m.saturating_sub(1))
        .ok()
        .and_then(|i| M.get(i).copied())
        .unwrap_or("")
}

/// Primeiro dia do mês seguinte / anterior a `d`.
fn mes_vizinho(d: NaiveDate, avancar: bool) -> NaiveDate {
    let (mut y, mut m) = (d.year(), d.month());
    if avancar {
        if m == 12 {
            y += 1;
            m = 1;
        } else {
            m += 1;
        }
    } else if m == 1 {
        y -= 1;
        m = 12;
    } else {
        m -= 1;
    }
    NaiveDate::from_ymd_opt(y, m, 1).unwrap_or(d)
}

/// Filtro por janela de tempo: um botão com o período atual que abre um CALENDÁRIO único para
/// escolher início e fim (clica no 1º dia, depois no 2º; o intervalo fica realçado).
#[component]
fn FiltroData(inicio: RwSignal<String>, fim: RwSignal<String>, ref_data: String) -> impl IntoView {
    let aberto = RwSignal::new(false);
    let inicial = parse_iso(&ref_data)
        .or_else(|| NaiveDate::from_ymd_opt(2026, 1, 1))
        .map(|d| NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap_or(d))
        .unwrap_or_default();
    let mes = RwSignal::new(inicial);
    let ativo = move || !inicio.get().is_empty() || !fim.get().is_empty();
    let rotulo = move || {
        let (i, f) = (inicio.get(), fim.get());
        if i.is_empty() && f.is_empty() {
            "Período".to_owned()
        } else {
            let a = if i.is_empty() {
                "…".to_owned()
            } else {
                fmt_data(&i)
            };
            let b = if f.is_empty() {
                "…".to_owned()
            } else {
                fmt_data(&f)
            };
            format!("{a} — {b}")
        }
    };
    // Clique num dia: começa novo intervalo, ou fecha o fim se já há início sem fim.
    let clicar = move |d: NaiveDate| {
        let iso_d = iso(d);
        let (i, f) = (inicio.get_untracked(), fim.get_untracked());
        if i.is_empty() || !f.is_empty() {
            inicio.set(iso_d);
            fim.set(String::new());
        } else if iso_d >= i {
            fim.set(iso_d);
        } else {
            inicio.set(iso_d);
        }
    };
    view! {
        <div class="filtro-data" class:filtro-data--aberto=move || aberto.get()>
            <button
                type="button"
                class="filtro-data__btn"
                class:filtro-data__btn--ativo=ativo
                aria-expanded=move || if aberto.get() { "true" } else { "false" }
                on:click=move |_| aberto.update(|a| *a = !*a)
            >
                <Icone arquivo="calendario.svg" />
                <span class="filtro-data__rotulo">{rotulo}</span>
            </button>
            <Show when=move || aberto.get() fallback=|| ()>
                <button
                    type="button"
                    class="filtro-data__fundo"
                    tabindex="-1"
                    aria-hidden="true"
                    on:click=move |_| aberto.set(false)
                ></button>
                <div class="filtro-data__pop">
                    <div class="cal__cab">
                        <button
                            type="button"
                            class="cal__nav"
                            aria-label="Mês anterior"
                            on:click=move |_| mes.update(|m| *m = mes_vizinho(*m, false))
                        >
                            <Icone arquivo="seta-esquerda.svg" />
                        </button>
                        <span class="cal__titulo">
                            {move || format!("{} {}", nome_mes(mes.get().month()), mes.get().year())}
                        </span>
                        <button
                            type="button"
                            class="cal__nav"
                            aria-label="Próximo mês"
                            on:click=move |_| mes.update(|m| *m = mes_vizinho(*m, true))
                        >
                            <Icone arquivo="seta-direita.svg" />
                        </button>
                    </div>
                    <div class="cal__semana">
                        {["dom", "seg", "ter", "qua", "qui", "sex", "sáb"]
                            .into_iter()
                            .map(|d| view! { <span>{d}</span> })
                            .collect_view()}
                    </div>
                    <div class="cal__grade">
                        {move || {
                            let base = mes.get();
                            let offset = base.weekday().num_days_from_sunday();
                            let ult = mes_vizinho(base, true);
                            let dias = (ult - base).num_days();
                            let (i, f) = (inicio.get(), fim.get());
                            let mut cels: Vec<AnyView> = Vec::new();
                            for _ in 0..offset {
                                cels.push(view! { <span class="cal__vazio"></span> }.into_any());
                            }
                            for n in 1..=dias {
                                let dia = NaiveDate::from_ymd_opt(base.year(), base.month(),
                                    u32::try_from(n).unwrap_or(1)).unwrap_or(base);
                                let s = iso(dia);
                                let eh_i = s == i;
                                let eh_f = s == f;
                                let tem_fim = !f.is_empty();
                                let mut classe = String::from("cal__dia");
                                if eh_i && !tem_fim {
                                    classe.push_str(" cal__dia--sel");
                                } else if eh_i {
                                    classe.push_str(" cal__dia--ini");
                                } else if eh_f {
                                    classe.push_str(" cal__dia--fim");
                                } else if tem_fim && s.as_str() > i.as_str() && s.as_str() < f.as_str() {
                                    classe.push_str(" cal__dia--mid");
                                }
                                cels.push(
                                    view! {
                                        <button type="button" class=classe on:click=move |_| clicar(dia)>
                                            {n.to_string()}
                                        </button>
                                    }
                                    .into_any(),
                                );
                            }
                            cels
                        }}
                    </div>
                    <button
                        type="button"
                        class="btn btn--secundario btn--sm filtro-data__limpar"
                        on:click=move |_| {
                            inicio.set(String::new());
                            fim.set(String::new());
                        }
                    >
                        "Limpar período"
                    </button>
                </div>
            </Show>
        </div>
    }
}

/// Histórico de produção: linha do tempo por data (fases do dia ligadas), com filtro e paginação.
#[component]
fn HistoricoProducao(ordens: Vec<OrdemProducao>) -> impl IntoView {
    let dados = StoredValue::new(ordens);
    let inicio = RwSignal::new(String::new());
    let fim = RwSignal::new(String::new());
    let desloc = RwSignal::new(0_i64);
    let limite = RwSignal::new(DATAS_POR_PAGINA);
    // Mudar a janela volta para a primeira página.
    Effect::new(move |_| {
        inicio.track();
        fim.track();
        desloc.set(0);
    });
    let vazio = dados.with_value(std::vec::Vec::is_empty);
    let ref_data = dados.with_value(|v| v.first().and_then(|o| o.data.clone()).unwrap_or_default());
    view! {
        <section class="prod-secao">
            <HistoricoCab titulo="Histórico de produção" inicio fim ref_data />
            {if vazio {
                view! { <p class="estado-vazio">"Sem ordens de produção registradas."</p> }
                    .into_any()
            } else {
                view! {
                    {move || {
                        let ordens = dados.get_value();
                        let pag = pagina_datas(
                            &ordens,
                            |o| o.data.clone().unwrap_or_default(),
                            &inicio.get(),
                            &fim.get(),
                            desloc.get(),
                        );
                        let dias = pag
                            .grupos
                            .into_iter()
                            .map(|(data, fases)| dia_producao(&data, &fases))
                            .collect_view();
                        view! {
                            <div class="mov-timeline">{dias}</div>
                            <PaginacaoBotoes limite deslocamento=desloc total=pag.total />
                        }
                    }}
                }
                    .into_any()
            }}
        </section>
    }
}

/// Uma linha (data) do histórico de produção: as ordens do dia ligadas por conectores.
fn dia_producao(data: &str, fases: &[&OrdemProducao]) -> impl IntoView {
    let mut itens: Vec<AnyView> = Vec::new();
    for (idx, o) in fases.iter().enumerate() {
        if idx > 0 {
            itens.push(view! { <span class="mov-liga"></span> }.into_any());
        }
        let status = o.status.clone().unwrap_or_default();
        let classe = format!("mov-fase mov-fase--prod-{}", status.to_lowercase());
        let lote = o.lote.map(|l| format!("Lote: {l}"));
        itens.push(
            view! {
                <div class=classe>
                    <span class="mov-fase__tipo">{rotulo_producao(&status)}</span>
                    <span class="mov-fase__qtd">{format!("{} un", fmt_milhar(o.quantidade))}</span>
                    {lote.map(|l| view! { <span class="mov-fase__meta">{l}</span> })}
                </div>
            }
            .into_any(),
        );
    }
    view! {
        <div class="mov-dia">
            <span class="mov-dia__data">{fmt_data(data)}</span>
            <div class="mov-fases">{itens}</div>
        </div>
    }
}

/// Histórico de movimentação: linha do tempo por data, com filtro e paginação.
#[component]
fn HistoricoMovimentacao(movimentos: Vec<Movimento>) -> impl IntoView {
    // Separação de venda não é movimento de saldo (quantidade 0) — fica fora da lista.
    let movimentos: Vec<Movimento> = movimentos
        .into_iter()
        .filter(|m| m.tipo != "SEPARACAO_VENDA")
        .collect();
    let dados = StoredValue::new(movimentos);
    let inicio = RwSignal::new(String::new());
    let fim = RwSignal::new(String::new());
    let desloc = RwSignal::new(0_i64);
    let limite = RwSignal::new(DATAS_POR_PAGINA);
    Effect::new(move |_| {
        inicio.track();
        fim.track();
        desloc.set(0);
    });
    let vazio = dados.with_value(std::vec::Vec::is_empty);
    let ref_data = dados.with_value(|v| v.first().map(|m| m.data.clone()).unwrap_or_default());
    view! {
        <section class="prod-secao">
            <HistoricoCab titulo="Histórico de movimentação" inicio fim ref_data />
            {if vazio {
                view! { <p class="estado-vazio">"Sem movimentações registradas."</p> }.into_any()
            } else {
                view! {
                    {move || {
                        let movs = dados.get_value();
                        let pag = pagina_datas(
                            &movs,
                            |m| m.data.clone(),
                            &inicio.get(),
                            &fim.get(),
                            desloc.get(),
                        );
                        let dias = pag
                            .grupos
                            .into_iter()
                            .map(|(data, fases)| dia_movimentacao(&data, &fases))
                            .collect_view();
                        view! {
                            <div class="mov-timeline">{dias}</div>
                            <PaginacaoBotoes limite deslocamento=desloc total=pag.total />
                        }
                    }}
                }
                    .into_any()
            }}
        </section>
    }
}

/// Uma linha (data) do histórico de movimentação: os movimentos do dia ligados por conectores.
fn dia_movimentacao(data: &str, fases: &[&Movimento]) -> impl IntoView {
    let mut itens: Vec<AnyView> = Vec::new();
    for (idx, m) in fases.iter().enumerate() {
        if idx > 0 {
            itens.push(view! { <span class="mov-liga"></span> }.into_any());
        }
        let entrada = m.quantidade >= 0;
        let classe = if m.tipo == "VENDA" {
            "mov-fase mov-fase--venda"
        } else if entrada {
            "mov-fase mov-fase--entrada"
        } else {
            "mov-fase mov-fase--saida"
        };
        let sinal = if entrada { "+" } else { "" };
        itens.push(
            view! {
                <div class=classe>
                    <span class="mov-fase__tipo">{rotulo_movimento(&m.tipo)}</span>
                    <span class="mov-fase__qtd">{format!("{sinal}{} un", fmt_milhar(m.quantidade))}</span>
                    <span class="mov-fase__meta">{format!("saldo {} un", fmt_milhar(m.saldo))}</span>
                </div>
            }
            .into_any(),
        );
    }
    view! {
        <div class="mov-dia">
            <span class="mov-dia__data">{fmt_data(data)}</span>
            <div class="mov-fases">{itens}</div>
        </div>
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
