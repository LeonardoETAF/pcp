//! Detalhe do Produto (doc 03 §4): cabeçalho com regra da classe, métricas e gráficos de 90 dias
//! (vendas e estoque), renderizados em SVG (Rust). Frontend burro (§3): tudo vem pronto da API.
//! Insights de IA e Solicitação de Produção entram no próximo passo (2.5 parte 2 / Fase 4).

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::api::{produto_detalhe, DetalheProduto, MetricasProduto, Ponto, RegraClasse};
use crate::contexto::Sessao;
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
                                        <p class="form-auth__erro">{e.to_string()}</p>
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

/// Conteúdo completo do detalhe (cabeçalho + regra + métricas + gráficos).
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
        <header class="prod-cab">
            <A href="/estoque" attr:class="btn btn--secundario btn--sm">
                "← Voltar"
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
        </header>

        {regra_classe(&d.classe, &d.regra, d.percentual_acumulado)}
        {metricas(&d.metricas)}

        <div class="prod-graficos">
            <section class="cartao">
                <header class="cartao__cab">
                    <h2 class="cartao__titulo">"Vendas diárias"</h2>
                    <p class="texto-suave">"Últimos 90 dias"</p>
                </header>
                <GraficoBarras dados=d.vendas_90d.clone() vazio="Sem vendas no período." />
            </section>
            <section class="cartao">
                <header class="cartao__cab">
                    <h2 class="cartao__titulo">"Evolução do estoque"</h2>
                    <p class="texto-suave">"Disponível — últimos 90 dias"</p>
                </header>
                <GraficoLinha dados=d.estoque_90d.clone() vazio="Sem snapshots no período." />
            </section>
        </div>
    }
}

/// Painel "regra da classe aplicada" (doc 03 §4.1): metas/limiar/fator + justificativa.
fn regra_classe(classe: &str, r: &RegraClasse, percentual: Option<f64>) -> impl IntoView {
    let limiar = r.limiar_critico_dias.map(|l| format!("Crítico ≤ {l} dias"));
    let pareto = percentual.map(|p| format!("Posição Pareto: {p:.1}%"));
    let classe = classe.to_owned();
    let fator = format!("{:.2}", r.fator_estoque);
    let meta = format!("{} dias", r.meta_cobertura_dias);
    let justificativa = r.justificativa.clone();
    view! {
        <section class="cartao prod-regra">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">{format!("Regra da classe {classe}")}</h2>
            </header>
            <p class="prod-regra__texto">{justificativa}</p>
            <div class="prod-regra__chips">
                <Pilula rotulo="Meta de cobertura" valor=meta />
                {limiar.map(|l| view! { <Pilula rotulo="Limiar" valor=l /> })}
                <Pilula rotulo="Fator de estoque" valor=fator />
                {pareto.map(|p| view! { <Pilula rotulo="Pareto" valor=p /> })}
            </div>
        </section>
    }
}

#[component]
fn Pilula(rotulo: &'static str, valor: String) -> impl IntoView {
    view! {
        <span class="pilula">
            <span class="pilula__rotulo">{rotulo}</span>
            <span class="pilula__valor">{valor}</span>
        </span>
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

/// Gráfico de barras (vendas diárias). SVG escalado ao máximo da série.
#[component]
#[allow(clippy::cast_precision_loss)] // séries curtas (≤90): conversão exata para f64
fn GraficoBarras(dados: Vec<Ponto>, vazio: &'static str) -> impl IntoView {
    const W: f64 = 680.0;
    const H: f64 = 160.0;
    if dados.is_empty() {
        return view! { <p class="estado-vazio">{vazio}</p> }.into_any();
    }
    let max = dados.iter().map(|p| p.valor).max().unwrap_or(1).max(1) as f64;
    let n = dados.len() as f64;
    let largura = W / n;
    let barras: Vec<_> = dados
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let altura = (p.valor as f64 / max) * H;
            let x = i as f64 * largura;
            view! {
                <rect
                    x=format!("{x:.2}")
                    y=format!("{:.2}", H - altura)
                    width=format!("{:.2}", (largura * 0.8).max(1.0))
                    height=format!("{altura:.2}")
                    class="graf__barra"
                />
            }
        })
        .collect();
    view! {
        <svg class="grafico" viewBox=format!("0 0 {W} {H}") preserveAspectRatio="none">
            {barras}
        </svg>
    }
    .into_any()
}

/// Gráfico de linha/área (evolução do estoque). SVG escalado ao máximo da série.
#[component]
#[allow(clippy::cast_precision_loss)]
fn GraficoLinha(dados: Vec<Ponto>, vazio: &'static str) -> impl IntoView {
    use std::fmt::Write as _;
    const W: f64 = 680.0;
    const H: f64 = 160.0;
    if dados.is_empty() {
        return view! { <p class="estado-vazio">{vazio}</p> }.into_any();
    }
    let max = dados.iter().map(|p| p.valor).max().unwrap_or(1).max(1) as f64;
    let n = dados.len();
    let passo = if n > 1 { W / (n as f64 - 1.0) } else { 0.0 };
    let coord = |i: usize, v: i64| (i as f64 * passo, H - (v as f64 / max) * H);
    let mut linha = String::new();
    for (i, p) in dados.iter().enumerate() {
        let (x, y) = coord(i, p.valor);
        let _ = write!(linha, "{x:.1},{y:.1} ");
    }
    let (ultimo_x, _) = coord(n - 1, 0);
    let area = format!("0,{H:.1} {linha}{ultimo_x:.1},{H:.1}");
    view! {
        <svg class="grafico" viewBox=format!("0 0 {W} {H}") preserveAspectRatio="none">
            <polygon points=area class="graf__area" />
            <polyline points=linha class="graf__linha" fill="none" />
        </svg>
    }
    .into_any()
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
