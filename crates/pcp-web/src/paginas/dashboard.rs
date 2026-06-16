//! Dashboard "Visão geral" no padrão do mockup, com dados REAIS do PCP (CLAUDE.md §0/§3/§13):
//! KPIs + donut (distribuição ABC) + barras (distribuição por status) + listas (estoque crítico
//! e a produzir). Sem OEE/ordens/expedições (não são do escopo do PCP). Frontend burro.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::api::{estoque, painel, Contagem, LinhaEstoque, PainelResumo};
use crate::contexto::Sessao;

fn cor_classe(classe: &str) -> &'static str {
    match classe {
        "A" => "var(--abc-a)",
        "B" => "var(--abc-b)",
        "C" => "var(--abc-c)",
        "D" => "var(--abc-d)",
        "F" => "var(--abc-f)",
        _ => "var(--abc-n)",
    }
}

fn conta_status(p: &PainelResumo, status: &str) -> i64 {
    p.por_status
        .iter()
        .find(|c| c.rotulo == status)
        .map_or(0, |c| c.quantidade)
}

#[component]
pub fn PaginaDashboard() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let painel_res = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            if let Some(t) = t {
                painel(t).await
            } else {
                Ok(PainelResumo::default())
            }
        },
    );
    let criticos = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => estoque(t, Some("critico".to_owned()), 6)
                    .await
                    .map(|p| p.itens),
                None => Ok(Vec::new()),
            }
        },
    );
    let produzir = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => estoque(t, None, 6).await.map(|p| p.itens),
                None => Ok(Vec::new()),
            }
        },
    );

    view! {
        <div class="painel">
            <Suspense fallback=|| view! { <p class="texto-suave">"Carregando painel…"</p> }>
                {move || {
                    painel_res
                        .get()
                        .map(|res| match res {
                            Err(e) => {
                                view! { <p class="form-auth__erro">{e.to_string()}</p> }.into_any()
                            }
                            Ok(p) => topo(&p).into_any(),
                        })
                }}
            </Suspense>
            <div class="painel__grade">
                <ListaProdutos
                    titulo="A produzir"
                    sub="Maiores sugestões de produção"
                    recurso=produzir
                    mostrar_sugestao=true
                />
                <ListaProdutos
                    titulo="Estoque crítico"
                    sub="Itens em estado crítico"
                    recurso=criticos
                    mostrar_sugestao=false
                />
            </div>
        </div>
    }
}

/// KPIs + gráficos (donut ABC + barras de status). Recebe o resumo já carregado.
fn topo(p: &PainelResumo) -> impl IntoView {
    let cobertura = p
        .cobertura_media
        .map_or_else(|| "—".to_owned(), |c| format!("{c:.1} d"));
    let cards = view! {
        <div class="kpis">
            <Kpi valor=p.total_produtos.to_string() rotulo="Produtos" sub="ativos no catálogo" />
            <Kpi
                valor=conta_status(p, "critico").to_string()
                rotulo="Estoque crítico"
                sub="abaixo do limiar da classe"
                realce="critico"
            />
            <Kpi valor=cobertura rotulo="Cobertura média" sub="dias (exclui sem histórico)" />
            <Kpi
                valor=p.total_sugerido.to_string()
                rotulo="A produzir"
                sub="soma das sugestões"
                realce="alto"
            />
        </div>
    };

    let abc: Vec<(String, i64, &'static str)> = p
        .por_classe
        .iter()
        .map(|c| (c.rotulo.clone(), c.quantidade, cor_classe(&c.rotulo)))
        .collect();
    let status = p.por_status.clone();

    view! {
        {cards}
        <div class="painel__graficos">
            <section class="cartao">
                <header class="cartao__cab">
                    <h2 class="cartao__titulo">"Distribuição ABC"</h2>
                    <p class="texto-suave">"Produtos por classe"</p>
                </header>
                <Donut dados=abc />
            </section>
            <section class="cartao">
                <header class="cartao__cab">
                    <h2 class="cartao__titulo">"Distribuição por status"</h2>
                    <p class="texto-suave">"Produtos por situação de estoque"</p>
                </header>
                <Barras dados=status />
            </section>
        </div>
    }
}

#[component]
fn Kpi(
    valor: String,
    rotulo: &'static str,
    sub: &'static str,
    #[prop(optional)] realce: &'static str,
) -> impl IntoView {
    let classe = if realce.is_empty() {
        "kpi".to_owned()
    } else {
        format!("kpi kpi--{realce}")
    };
    view! {
        <div class=classe>
            <span class="kpi__valor">{valor}</span>
            <span class="kpi__rotulo">{rotulo}</span>
            <span class="kpi__sub">{sub}</span>
        </div>
    }
}

#[component]
#[allow(clippy::cast_precision_loss)] // contagens pequenas: conversão exata para f64
fn Donut(dados: Vec<(String, i64, &'static str)>) -> impl IntoView {
    const CX: f64 = 80.0;
    const R: f64 = 58.0;
    const SW: f64 = 22.0;
    let circ = 2.0 * std::f64::consts::PI * R;
    let total: i64 = dados.iter().map(|d| d.1).sum();

    if total == 0 {
        return view! { <p class="estado-vazio">"Sem dados."</p> }.into_any();
    }

    let mut acc = 0.0_f64;
    let fatias: Vec<_> = dados
        .iter()
        .filter(|d| d.1 > 0)
        .map(|(_, v, cor)| {
            let len = (*v as f64 / total as f64) * circ;
            let off = -acc;
            acc += len;
            view! {
                <circle
                    cx=CX
                    cy=CX
                    r=R
                    fill="none"
                    stroke-width=SW
                    stroke-dasharray=format!("{len:.2} {:.2}", circ - len)
                    stroke-dashoffset=format!("{off:.2}")
                    transform=format!("rotate(-90 {CX} {CX})")
                    style=format!("stroke:{cor}")
                />
            }
        })
        .collect();

    let legenda: Vec<_> = dados
        .iter()
        .filter(|d| d.1 > 0)
        .map(|(rot, v, cor)| {
            view! {
                <li class="legenda__item">
                    <span class="legenda__cor" style=format!("background:{cor}")></span>
                    <span class="legenda__rotulo">{rot.clone()}</span>
                    <span class="legenda__valor">{*v}</span>
                </li>
            }
        })
        .collect();

    view! {
        <div class="donut">
            <svg class="donut__svg" viewBox="0 0 160 160">
                <circle cx=CX cy=CX r=R fill="none" stroke-width=SW class="donut__trilho" />
                {fatias}
                <text x=CX y="74" class="donut__num">{total}</text>
                <text x=CX y="94" class="donut__leg">"produtos"</text>
            </svg>
            <ul class="legenda">{legenda}</ul>
        </div>
    }
    .into_any()
}

#[component]
#[allow(clippy::cast_precision_loss)]
fn Barras(dados: Vec<Contagem>) -> impl IntoView {
    let maximo = dados.iter().map(|c| c.quantidade).max().unwrap_or(1).max(1);
    if dados.is_empty() {
        return view! { <p class="estado-vazio">"Sem dados."</p> }.into_any();
    }
    let linhas: Vec<_> = dados
        .into_iter()
        .map(|c| {
            let pct = (c.quantidade as f64 / maximo as f64 * 100.0).round();
            view! {
                <div class="barra-linha">
                    <span class="barra-linha__rotulo">{c.rotulo}</span>
                    <span class="barra-linha__trilho">
                        <span class="barra-linha__preenche" style=format!("width:{pct}%")></span>
                    </span>
                    <span class="barra-linha__valor">{c.quantidade}</span>
                </div>
            }
        })
        .collect();
    view! { <div class="barras">{linhas}</div> }.into_any()
}

#[component]
fn ListaProdutos(
    titulo: &'static str,
    sub: &'static str,
    recurso: Resource<Result<Vec<LinhaEstoque>, ServerFnError>>,
    mostrar_sugestao: bool,
) -> impl IntoView {
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <div>
                    <h2 class="cartao__titulo">{titulo}</h2>
                    <p class="texto-suave">{sub}</p>
                </div>
                <A href="/estoque" attr:class="link-ver-todos">
                    "Ver estoque"
                </A>
            </header>
            <Suspense fallback=|| view! { <p class="texto-suave">"Carregando…"</p> }>
                {move || {
                    recurso
                        .get()
                        .map(|res| match res {
                            Err(e) => {
                                view! { <p class="form-auth__erro">{e.to_string()}</p> }.into_any()
                            }
                            Ok(itens) if itens.is_empty() => {
                                view! { <p class="estado-vazio">"Nada por aqui."</p> }.into_any()
                            }
                            Ok(itens) => {
                                view! {
                                    <ul class="lista-prod">
                                        {itens
                                            .into_iter()
                                            .map(|i| {
                                                linha_produto(&i, mostrar_sugestao)
                                            })
                                            .collect_view()}
                                    </ul>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>
        </section>
    }
}

fn linha_produto(i: &LinhaEstoque, mostrar_sugestao: bool) -> impl IntoView {
    let nome = i
        .produto
        .clone()
        .unwrap_or_else(|| i.codigo_estoque.clone());
    let href = format!("/estoque/{}", i.codigo_estoque);
    let metrica = if mostrar_sugestao {
        format!("{} un", i.qtd_sugerida)
    } else {
        format!("{:.1} d", i.cobertura_dias)
    };
    view! {
        <li class="lista-prod__item">
            <span class=format!("badge badge--abc-{}", i.classe.to_lowercase())>
                {i.classe.clone()}
            </span>
            <div class="lista-prod__nome">
                <A href=href attr:class="lista-prod__link">
                    {nome}
                </A>
                <span class="lista-prod__codigo">{i.codigo_estoque.clone()}</span>
            </div>
            <span class="lista-prod__metrica">{metrica}</span>
        </li>
    }
}
