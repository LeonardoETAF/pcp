//! Dashboard "Visão geral" no padrão do mockup, com dados REAIS do PCP (CLAUDE.md §0/§3/§13):
//! KPIs + donut (distribuição ABC) + barras (distribuição por status) + listas (estoque crítico
//! e a produzir). Sem OEE/ordens/expedições (não são do escopo do PCP). Frontend burro.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::api::{
    alertas, dashboard_classes, estoque, painel, AlertaResumo, ClasseResumo, ConsultaEstoque,
    Contagem, LinhaEstoque, PainelResumo,
};
use crate::contexto::Sessao;
use crate::formato::{fmt_cobertura, fmt_milhar, nome_exibicao, rotulo_status};

/// Cor de criticidade do KPI (doc 02 §9.2): vermelho/amarelo conforme o threshold.
#[allow(clippy::cast_precision_loss)] // contagens pequenas: conversão exata para f64
fn realce_criticos(criticos: i64, total: i64) -> &'static str {
    if total == 0 {
        return "";
    }
    let pct = (criticos as f64 / total as f64) * 100.0;
    if pct > 20.0 {
        "critico"
    } else if pct > 10.0 {
        "medio"
    } else {
        ""
    }
}

/// Cor de criticidade da cobertura média (doc 02 §9.2): < 30 dias vermelho, < 60 amarelo.
fn realce_cobertura(c: Option<f64>) -> &'static str {
    match c {
        Some(v) if v < 30.0 => "critico",
        Some(v) if v < 60.0 => "medio",
        _ => "",
    }
}

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
#[allow(clippy::too_many_lines)] // wiring de Resources + markup declarativo
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
                Some(t) => estoque(
                    t,
                    ConsultaEstoque {
                        status: Some("critico".to_owned()),
                        limite: 6,
                        ..Default::default()
                    },
                )
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
                Some(t) => estoque(
                    t,
                    ConsultaEstoque {
                        ordem: Some("sugerida_desc".to_owned()),
                        limite: 6,
                        ..Default::default()
                    },
                )
                .await
                .map(|p| p.itens),
                None => Ok(Vec::new()),
            }
        },
    );
    let classes_res = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => dashboard_classes(t).await.unwrap_or_default(),
                None => Vec::new(),
            }
        },
    );
    let alertas_res = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => alertas(t).await.unwrap_or_default(),
                None => Vec::new(),
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
            <Suspense fallback=|| view! { <p class="texto-suave">"Carregando metas…"</p> }>
                {move || {
                    let classes = classes_res.get().unwrap_or_default();
                    view! { <SecaoClasses classes /> }
                }}
            </Suspense>
            <div class="painel__base">
                <TabelaProduzir recurso=produzir />
                <div class="painel__lateral">
                    <EstoqueCritico recurso=criticos />
                    <AlertasRecentes recurso=alertas_res />
                </div>
            </div>
        </div>
    }
}

/// KPIs + gráficos (donut ABC + barras de status). Recebe o resumo já carregado.
fn topo(p: &PainelResumo) -> impl IntoView {
    let cobertura = p
        .cobertura_media
        .map_or_else(|| "0".to_owned(), |c| format!("{c:.1}"));
    let criticos = conta_status(p, "critico");
    let cards = view! {
        <div class="kpis">
            <Kpi
                valor=p.total_produtos.to_string()
                rotulo="Produtos"
                sub="Ativos no catálogo"
                icone="estoque-inventario.svg"
            />
            <Kpi
                valor=criticos.to_string()
                rotulo="Estoque crítico"
                sub="Abaixo do limiar da classe"
                icone="alerta.svg"
                realce=realce_criticos(criticos, p.total_produtos)
            />
            <Kpi
                valor=cobertura
                rotulo="Cobertura média"
                sub="Dias"
                icone="relogio.svg"
                realce=realce_cobertura(p.cobertura_media)
            />
            <Kpi
                valor=p.total_sugerido.to_string()
                rotulo="A produzir"
                sub="Soma das sugestões"
                icone="ordens-producao.svg"
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

/// Painel de metas físicas ABC (doc 02 §9.1) + cobertura média por classe (doc 03 §2).
#[component]
fn SecaoClasses(classes: Vec<ClasseResumo>) -> impl IntoView {
    if classes.is_empty() {
        return ().into_any();
    }
    let metas: Vec<_> = classes
        .iter()
        .filter(|c| c.pct_fisico_meta.is_some())
        .cloned()
        .collect();
    let cobertura: Vec<_> = classes.clone();
    view! {
        <div class="painel__graficos">
            <section class="cartao">
                <header class="cartao__cab">
                    <h2 class="cartao__titulo">"Metas de estoque físico (ABC)"</h2>
                    <p class="texto-suave">"Participação real × meta (±3 p.p.)"</p>
                </header>
                <div class="metas-fisicas">
                    {metas
                        .into_iter()
                        .map(|c| view! { <MetaFisica c /> })
                        .collect_view()}
                </div>
            </section>
            <section class="cartao">
                <header class="cartao__cab">
                    <h2 class="cartao__titulo">"Cobertura por classe"</h2>
                    <p class="texto-suave">"Dias"</p>
                </header>
                <div class="cobertura-classes">
                    {cobertura
                        .into_iter()
                        .map(|c| {
                            let cob = c
                                .cobertura_media
                                .map_or_else(|| "—".to_owned(), |v| format!("{v:.1}"));
                            view! {
                                <div class="cob-classe">
                                    <span class=format!("badge badge--abc-{}", c.classe.to_lowercase())>
                                        {c.classe.clone()}
                                    </span>
                                    <span class="cob-classe__valor">{cob}</span>
                                </div>
                            }
                        })
                        .collect_view()}
                </div>
            </section>
        </div>
    }
    .into_any()
}

#[component]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // pct 0..100 p/ largura da barra
fn MetaFisica(c: ClasseResumo) -> impl IntoView {
    let meta = c.pct_fisico_meta.unwrap_or(0);
    let atingida = c.meta_atingida.unwrap_or(false);
    let largura = c.pct_fisico_real.clamp(0.0, 100.0);
    let status = if atingida {
        "Meta atingida"
    } else {
        "Fora da meta"
    };
    let classe_status = if atingida {
        "meta-chip meta-chip--ok"
    } else {
        "meta-chip meta-chip--fora"
    };
    view! {
        <div class="meta-fisica">
            <div class="meta-fisica__topo">
                <span class=format!("badge badge--abc-{}", c.classe.to_lowercase())>
                    {c.classe.clone()}
                </span>
                <span class="meta-fisica__pct">
                    {format!("{:.1}% / meta {}%", c.pct_fisico_real, meta)}
                </span>
                <span class=classe_status>{status}</span>
            </div>
            <div class="meta-fisica__trilho">
                <span class="meta-fisica__preenche" style=format!("width:{largura}%")></span>
                <span class="meta-fisica__marca" style=format!("left:{meta}%")></span>
            </div>
        </div>
    }
}

/// Alertas mais recentes (doc 03 §2): fila resumida, link para a Central.
#[component]
fn AlertasRecentes(recurso: Resource<Vec<AlertaResumo>>) -> impl IntoView {
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <div>
                    <h2 class="cartao__titulo">"Alertas recentes"</h2>
                    <p class="texto-suave">"Itens em alerta de produção"</p>
                </div>
                <A href="/alertas" attr:class="link-ver-todos">
                    "Ver alertas"
                </A>
            </header>
            <Suspense fallback=|| view! { <p class="texto-suave">"Carregando…"</p> }>
                {move || {
                    let itens = recurso.get().unwrap_or_default();
                    if itens.is_empty() {
                        view! { <p class="estado-vazio">"Nenhum alerta no momento."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="lista-prod">
                                {itens
                                    .into_iter()
                                    .take(5)
                                    .map(|a| {
                                        let nome = a
                                            .produto
                                            .clone()
                                            .unwrap_or_else(|| a.codigo_estoque.clone());
                                        let href = format!("/estoque/{}", a.codigo_estoque);
                                        view! {
                                            <li class="lista-prod__item">
                                                <span class=format!(
                                                    "badge badge--prio-{}",
                                                    a.prioridade,
                                                )>{a.prioridade.clone()}</span>
                                                <div class="lista-prod__nome">
                                                    <A href=href attr:class="lista-prod__link">
                                                        {nome}
                                                    </A>
                                                    <span class="lista-prod__codigo">
                                                        {a.codigo_estoque.clone()}
                                                    </span>
                                                </div>
                                                <span class="lista-prod__metrica">
                                                    {fmt_milhar(a.qtd_sugerida)}
                                                </span>
                                            </li>
                                        }
                                    })
                                    .collect_view()}
                            </ul>
                        }
                            .into_any()
                    }
                }}
            </Suspense>
        </section>
    }
}

#[component]
fn Kpi(
    valor: String,
    rotulo: &'static str,
    sub: &'static str,
    icone: &'static str,
    #[prop(optional)] realce: &'static str,
) -> impl IntoView {
    let classe = if realce.is_empty() {
        "kpi".to_owned()
    } else {
        format!("kpi kpi--{realce}")
    };
    let estilo = format!("-webkit-mask-image:url(/icons/{icone});mask-image:url(/icons/{icone})");
    view! {
        <div class=classe>
            <span class="kpi__chip">
                <span class="icone-mask" style=estilo></span>
            </span>
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

/// "A produzir" como tabela (estilo editorial): maiores sugestões de produção (doc 03 §2).
#[component]
fn TabelaProduzir(recurso: Resource<Result<Vec<LinhaEstoque>, ServerFnError>>) -> impl IntoView {
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <div>
                    <h2 class="cartao__titulo">"A produzir"</h2>
                    <p class="texto-suave">"Maiores sugestões de produção"</p>
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
                                    <div class="tabela-rolavel">
                                        <table class="tabela tabela--centro">
                                            <thead>
                                                <tr>
                                                    <th>"Código"</th>
                                                    <th>"Produto"</th>
                                                    <th>"Classe"</th>
                                                    <th>"Disponível"</th>
                                                    <th>"Recomendada"</th>
                                                    <th>"Produzir"</th>
                                                    <th>"Status"</th>
                                                </tr>
                                            </thead>
                                            <tbody>
                                                {itens
                                                    .into_iter()
                                                    .map(|i| view! { <LinhaProduzir i /> })
                                                    .collect_view()}
                                            </tbody>
                                        </table>
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

#[component]
fn LinhaProduzir(i: LinhaEstoque) -> impl IntoView {
    let nome = nome_exibicao(
        i.produto.as_deref(),
        i.configuracao.as_deref(),
        &i.codigo_estoque,
    );
    let href = format!("/estoque/{}", i.codigo_estoque);
    view! {
        <tr>
            <td class="tabela__cod">
                <A href=href attr:class="lista-prod__link">
                    {i.codigo_estoque.clone()}
                </A>
            </td>
            <td>{nome}</td>
            <td>
                <span class=format!("badge badge--abc-{}", i.classe.to_lowercase())>
                    {i.classe.clone()}
                </span>
            </td>
            <td class="tabela__num">{fmt_milhar(i.qtd_disponivel)}</td>
            <td class="tabela__num">{fmt_milhar(i.estoque_total_recomendado)}</td>
            <td class="tabela__num tabela__produzir">{fmt_milhar(i.qtd_sugerida)}</td>
            <td>
                <span class=format!("badge badge--status-{}", i.status)>
                    {rotulo_status(&i.status)}
                </span>
            </td>
        </tr>
    }
}

/// Lista compacta de estoque crítico (ícone de caixa + nome/código + disponível/cobertura).
#[component]
fn EstoqueCritico(recurso: Resource<Result<Vec<LinhaEstoque>, ServerFnError>>) -> impl IntoView {
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <div>
                    <h2 class="cartao__titulo">"Estoque crítico"</h2>
                    <p class="texto-suave">"Itens em estado crítico"</p>
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
                                view! { <p class="estado-vazio">"Nenhum item crítico."</p> }
                                    .into_any()
                            }
                            Ok(itens) => {
                                view! {
                                    <ul class="lista-critico">
                                        {itens
                                            .into_iter()
                                            .map(|i| view! { <ItemCritico i /> })
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

#[component]
fn ItemCritico(i: LinhaEstoque) -> impl IntoView {
    let nome = nome_exibicao(
        i.produto.as_deref(),
        i.configuracao.as_deref(),
        &i.codigo_estoque,
    );
    let estilo = "-webkit-mask-image:url(/icons/estoque-inventario.svg);\
                  mask-image:url(/icons/estoque-inventario.svg)";
    view! {
        <li class="critico-item">
            <span class="critico-item__icone">
                <span class="icone-mask" style=estilo></span>
            </span>
            <div class="critico-item__nome">
                <A href=format!("/estoque/{}", i.codigo_estoque) attr:class="lista-prod__link">
                    {nome}
                </A>
                <span class="lista-prod__codigo">{i.codigo_estoque.clone()}</span>
            </div>
            <div class="critico-item__valores">
                <span class="critico-item__qtd">{fmt_milhar(i.qtd_disponivel)}" un"</span>
                <span class="critico-item__cob">{fmt_cobertura(i.cobertura_dias)}" d"</span>
            </div>
        </li>
    }
}
