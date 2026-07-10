//! Dashboard "Visão geral" no padrão do mockup, com dados REAIS do PCP (CLAUDE.md §0/§3/§13):
//! KPIs + donut (distribuição ABC) + barras (distribuição por status) + listas (estoque crítico
//! e a produzir). Sem OEE/ordens/expedições (não são do escopo do PCP). Frontend burro.

use leptos::prelude::*;
use leptos_router::components::A;

use crate::api::{
    dashboard_classes, estoque, painel, vendas_mensais, ClasseResumo, ConsultaEstoque, Contagem,
    LinhaEstoque, PainelResumo, VendaMes,
};
use crate::contexto::Sessao;
use crate::erro::mensagem_usuario;
use crate::formato::{
    cor_status, fmt_cobertura, fmt_compacto, fmt_dec1, fmt_milhar, nome_exibicao, rotulo_status,
};

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

/// Abreviação pt-BR do mês (1–12) para o eixo do gráfico de vendas.
fn mes_abrev(mes: i32) -> &'static str {
    const M: [&str; 12] = [
        "Jan", "Fev", "Mar", "Abr", "Mai", "Jun", "Jul", "Ago", "Set", "Out", "Nov", "Dez",
    ];
    usize::try_from(mes - 1)
        .ok()
        .and_then(|i| M.get(i))
        .copied()
        .unwrap_or("—")
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
    let vendas_res = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => vendas_mensais(t).await.unwrap_or_default(),
                None => Vec::new(),
            }
        },
    );

    view! {
        <div class="painel">
            // Linha 1 — KPIs (A produzir em destaque, Produtos, Crítico, gauge de cobertura).
            <Suspense fallback=|| view! { <p class="texto-suave">"Carregando painel…"</p> }>
                {move || {
                    painel_res
                        .get()
                        .map(|res| match res {
                            Err(e) => {
                                view! { <p class="form-auth__erro">{mensagem_usuario(&e)}</p> }.into_any()
                            }
                            Ok(p) => kpis_topo(&p).into_any(),
                        })
                }}
            </Suspense>
            // Linha 2 — Vendas por mês (real) + cobertura por classe (anéis).
            <div class="painel__graficos">
                <Suspense fallback=|| view! { <CartaoVazio titulo="Vendas por mês" /> }>
                    {move || {
                        let dados = vendas_res.get().unwrap_or_default();
                        view! { <VendasMes dados /> }
                    }}
                </Suspense>
                <Suspense fallback=|| view! { <CartaoVazio titulo="Cobertura por classe" /> }>
                    {move || {
                        let classes = classes_res.get().unwrap_or_default();
                        view! { <CoberturaRings classes /> }
                    }}
                </Suspense>
            </div>
            // Linha 3 — Distribuição ABC (estreita) + Estoque crítico (larga).
            <div class="painel__graficos painel__graficos--abc">
                <Suspense fallback=|| view! { <CartaoVazio titulo="Distribuição ABC" /> }>
                    {move || {
                        painel_res
                            .get()
                            .and_then(Result::ok)
                            .map(|p| view! { <CartaoDonut por_classe=p.por_classe /> })
                    }}
                </Suspense>
                <EstoqueCritico recurso=criticos />
            </div>
            // Linha 4 — Metas físicas (ABC) + distribuição por status.
            <div class="painel__graficos">
                <Suspense fallback=|| view! { <CartaoVazio titulo="Metas de estoque físico (ABC)" /> }>
                    {move || {
                        let classes = classes_res.get().unwrap_or_default();
                        view! { <SecaoMetas classes /> }
                    }}
                </Suspense>
                <Suspense fallback=|| view! { <CartaoVazio titulo="Distribuição por status" /> }>
                    {move || {
                        painel_res
                            .get()
                            .and_then(Result::ok)
                            .map(|p| view! { <CartaoStatus por_status=p.por_status /> })
                    }}
                </Suspense>
            </div>
            // Linha 5 — Maiores sugestões de produção (largura cheia).
            <TabelaProduzir recurso=produzir />
        </div>
    }
}

/// Cartão vazio de carregamento (skeleton leve, §16 — nenhuma seção "toda branca").
#[component]
fn CartaoVazio(titulo: &'static str) -> impl IntoView {
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">{titulo}</h2>
            </header>
            <p class="texto-suave">"Carregando…"</p>
        </section>
    }
}

/// Linha de KPIs: "A produzir" em destaque (card escuro), Produtos, Crítico e gauge de cobertura.
fn kpis_topo(p: &PainelResumo) -> impl IntoView {
    let criticos = conta_status(p, "critico");
    view! {
        <div class="kpis">
            <Kpi
                valor=fmt_compacto(p.total_sugerido)
                rotulo="A produzir"
                sub="soma das sugestões"
                hero=true
            />
            <Kpi valor=fmt_milhar(p.total_produtos) rotulo="Produtos" sub="ativos" />
            <Kpi
                valor=fmt_milhar(criticos)
                rotulo="Crítico"
                sub="abaixo do limiar"
                realce=realce_criticos(criticos, p.total_produtos)
            />
            <KpiGauge valor=p.cobertura_media />
        </div>
    }
}

/// Cartão da distribuição ABC (donut + legenda horizontal). §12: cor fixa por classe.
#[component]
fn CartaoDonut(por_classe: Vec<Contagem>) -> impl IntoView {
    let abc: Vec<(String, i64, &'static str)> = por_classe
        .iter()
        .map(|c| (c.rotulo.clone(), c.quantidade, cor_classe(&c.rotulo)))
        .collect();
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Distribuição ABC"</h2>
                <p class="texto-suave">"Produtos por classe"</p>
            </header>
            <Donut dados=abc />
        </section>
    }
}

/// Cartão da distribuição por status (barras minimalistas com knob; semáforo §12).
#[component]
fn CartaoStatus(por_status: Vec<Contagem>) -> impl IntoView {
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Distribuição por status"</h2>
                <p class="texto-suave">"Produtos por situação de estoque"</p>
            </header>
            <Barras dados=por_status />
        </section>
    }
}

/// Cartão de metas físicas ABC (doc 02 §9.1): participação real × meta por classe.
#[component]
fn SecaoMetas(classes: Vec<ClasseResumo>) -> impl IntoView {
    let metas: Vec<_> = classes
        .into_iter()
        .filter(|c| c.pct_fisico_meta.is_some())
        .collect();
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Metas de estoque físico (ABC)"</h2>
                <p class="texto-suave">"Participação real × meta (±3 p.p.)"</p>
            </header>
            <div class="metas-fisicas">
                {metas.into_iter().map(|c| view! { <MetaFisica c /> }).collect_view()}
            </div>
        </section>
    }
}

#[component]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // pct 0..100 p/ largura da barra
fn MetaFisica(c: ClasseResumo) -> impl IntoView {
    let meta = c.pct_fisico_meta.unwrap_or(0);
    let largura = c.pct_fisico_real.clamp(0.0, 100.0);
    let sem_meta = meta == 0;
    let texto_pct = if sem_meta {
        format!("{}% / sem meta", fmt_dec1(c.pct_fisico_real))
    } else {
        format!("{}% / meta {meta}%", fmt_dec1(c.pct_fisico_real))
    };
    view! {
        <div class="meta-fisica">
            <span class=format!("badge badge--abc-{}", c.classe.to_lowercase())>
                {c.classe.clone()}
            </span>
            <div class="meta-fisica__trilho">
                <span class="meta-fisica__preenche" style=format!("width:{largura}%")></span>
                {(!sem_meta)
                    .then(|| {
                        view! {
                            <span class="meta-fisica__marca" style=format!("left:{meta}%")></span>
                        }
                    })}
            </div>
            <span class="meta-fisica__pct">{texto_pct}</span>
        </div>
    }
}

/// KPI no estilo editorial do mockup: rótulo em caixa-alta (topo), número grande, sub embaixo.
/// `hero` deixa o card escuro (destaque de "A produzir"); `realce` pinta o número (semáforo §12).
#[component]
fn Kpi(
    valor: String,
    rotulo: &'static str,
    sub: &'static str,
    #[prop(optional)] realce: &'static str,
    #[prop(optional)] hero: bool,
) -> impl IntoView {
    let mut classe = String::from("kpi");
    if hero {
        classe.push_str(" kpi--hero");
    }
    if !realce.is_empty() {
        classe.push_str(" kpi--");
        classe.push_str(realce);
    }
    view! {
        <div class=classe>
            <span class="kpi__rotulo">{rotulo}</span>
            <span class="kpi__valor">{valor}</span>
            <span class="kpi__sub">{sub}</span>
        </div>
    }
}

/// Escala visual (dias) do gauge de cobertura — teto documentado de cobertura (doc 02 §11).
/// É só eixo de apresentação; o número e a cor vêm da API/regra (frontend burro, §3).
const GAUGE_MAX_DIAS: f64 = 60.0;

/// KPI de cobertura média como gauge semicircular. Cor do arco pela criticidade (semáforo §12).
#[component]
#[allow(clippy::cast_possible_truncation)] // ângulo 0..180 para o path do arco
fn KpiGauge(valor: Option<f64>) -> impl IntoView {
    // Semicírculo de raio 52 centrado em (60,60): da esquerda (180°) à direita (0°).
    const R: f64 = 52.0;
    const CX: f64 = 60.0;
    let dias = valor.unwrap_or(0.0);
    let realce = realce_cobertura(valor);
    let frac = (dias / GAUGE_MAX_DIAS).clamp(0.0, 1.0);
    let ponto = |t: f64| {
        let ang = std::f64::consts::PI * (1.0 - t); // t=0 → 180°, t=1 → 0°
        (CX + R * ang.cos(), CX - R * ang.sin())
    };
    let (x0, y0) = ponto(0.0);
    let (x1, y1) = ponto(frac);
    let arco_fundo = format!("M {x0:.2} {y0:.2} A {R} {R} 0 0 1 {:.2} {:.2}", CX + R, CX);
    let arco_valor = format!(
        "M {x0:.2} {y0:.2} A {R} {R} 0 {} 1 {x1:.2} {y1:.2}",
        i32::from(frac > 0.5)
    );
    let classe_num = if realce.is_empty() {
        "gauge__num".to_owned()
    } else {
        format!("gauge__num gauge__num--{realce}")
    };
    view! {
        <div class="kpi kpi--gauge">
            <svg class="gauge__svg" viewBox="0 0 120 72">
                <path class="gauge__fundo" d=arco_fundo fill="none" />
                <path class=format!("gauge__arco gauge__arco--{realce}") d=arco_valor fill="none" />
                <text x=CX y="58" class=classe_num>{fmt_dec1(dias)}</text>
            </svg>
            <span class="kpi__rotulo kpi__rotulo--gauge">"Cobertura média"</span>
            <span class="kpi__sub">"dias"</span>
        </div>
    }
}

/// Gráfico de barras de vendas por mês (dado real de `vendas_dia`). Última barra destacada;
/// unidade adaptativa (mi/mil/un.) conforme a magnitude. Frontend burro: só exibe (§3).
#[component]
#[allow(clippy::cast_precision_loss)] // totais mensais cabem em f64 sem perda relevante
fn VendasMes(dados: Vec<VendaMes>) -> impl IntoView {
    if dados.is_empty() {
        return view! {
            <section class="cartao">
                <header class="cartao__cab">
                    <h2 class="cartao__titulo">"Vendas por mês"</h2>
                </header>
                <p class="estado-vazio">"Sem vendas no período."</p>
            </section>
        }
        .into_any();
    }
    let maximo = dados.iter().map(|d| d.total).max().unwrap_or(1).max(1) as f64;
    let (divisor, unidade) = if maximo >= 1_000_000.0 {
        (1_000_000.0, "mi un.")
    } else if maximo >= 1_000.0 {
        (1_000.0, "mil un.")
    } else {
        (1.0, "un.")
    };
    let ultimo = dados.len() - 1;
    let barras: Vec<_> = dados
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let altura = (d.total as f64 / maximo * 100.0).round();
            let classe = if i == ultimo {
                "barra-mes barra-mes--destaque"
            } else {
                "barra-mes"
            };
            view! {
                <div class=classe>
                    <span class="barra-mes__valor">{fmt_dec1(d.total as f64 / divisor)}</span>
                    <span class="barra-mes__col" style=format!("height:{altura}%")></span>
                    <span class="barra-mes__mes">{mes_abrev(d.mes)}</span>
                </div>
            }
        })
        .collect();
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Vendas por mês"</h2>
                <p class="texto-suave">{unidade}</p>
            </header>
            <div class="grafico-mes">{barras}</div>
        </section>
    }
    .into_any()
}

/// Anéis de cobertura por classe: preenchimento = cobertura ÷ meta da classe (clamp 0–1).
/// Cor fixa por classe ABC (§12). Meta vem da config via API (frontend burro, §3).
#[component]
fn CoberturaRings(classes: Vec<ClasseResumo>) -> impl IntoView {
    if classes.is_empty() {
        return view! {
            <section class="cartao">
                <header class="cartao__cab">
                    <h2 class="cartao__titulo">"Cobertura por classe"</h2>
                </header>
                <p class="estado-vazio">"Sem dados."</p>
            </section>
        }
        .into_any();
    }
    let rings: Vec<_> = classes
        .iter()
        .map(|c| {
            let cob = c.cobertura_media.unwrap_or(0.0);
            let meta = f64::from(c.cobertura_meta_dias.max(1));
            let frac = (cob / meta).clamp(0.0, 1.0);
            let valor = c.cobertura_media.map_or_else(|| "—".to_owned(), fmt_dec1);
            view! {
                <Ring classe=c.classe.clone() frac cor=cor_classe(&c.classe).to_owned() valor />
            }
        })
        .collect();
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Cobertura por classe"</h2>
                <p class="texto-suave">"dias · anel = % da meta"</p>
            </header>
            <div class="rings">{rings}</div>
        </section>
    }
    .into_any()
}

/// Anel individual de cobertura de uma classe (SVG). `frac` 0–1 preenche o arco.
#[component]
fn Ring(classe: String, frac: f64, cor: String, valor: String) -> impl IntoView {
    const R: f64 = 26.0;
    let circ = 2.0 * std::f64::consts::PI * R;
    let preenche = circ * frac;
    view! {
        <div class="ring">
            <svg class="ring__svg" viewBox="0 0 64 64">
                <circle cx="32" cy="32" r=R fill="none" stroke-width="7" class="ring__trilho" />
                <circle
                    cx="32"
                    cy="32"
                    r=R
                    fill="none"
                    stroke-width="7"
                    stroke-linecap="round"
                    stroke-dasharray=format!("{preenche:.2} {:.2}", circ - preenche)
                    transform="rotate(-90 32 32)"
                    style=format!("stroke:{cor}")
                />
                <text x="32" y="37" class="ring__num">{valor}</text>
            </svg>
            <span class="ring__classe">{classe}</span>
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

    // Legenda horizontal (mockup): swatch + letra + contagem, em grade que quebra linha.
    let legenda: Vec<_> = dados
        .iter()
        .filter(|d| d.1 > 0)
        .map(|(rot, v, cor)| {
            view! {
                <li class="legenda__item">
                    <span class="legenda__cor" style=format!("background:{cor}")></span>
                    <span class="legenda__rotulo">{rot.clone()}</span>
                    <span class="legenda__valor">{fmt_milhar(*v)}</span>
                </li>
            }
        })
        .collect();

    view! {
        <div class="donut donut--coluna">
            <svg class="donut__svg" viewBox="0 0 160 160">
                <circle cx=CX cy=CX r=R fill="none" stroke-width=SW class="donut__trilho" />
                {fatias}
                <text x=CX y="74" class="donut__num">{fmt_milhar(total)}</text>
                <text x=CX y="94" class="donut__leg">"produtos"</text>
            </svg>
            <ul class="legenda legenda--grade">{legenda}</ul>
        </div>
    }
    .into_any()
}

#[component]
#[allow(clippy::cast_precision_loss)]
fn Barras(dados: Vec<Contagem>) -> impl IntoView {
    if dados.is_empty() {
        return view! { <p class="estado-vazio">"Sem dados."</p> }.into_any();
    }
    let maximo = dados.iter().map(|c| c.quantidade).max().unwrap_or(1).max(1);
    let linhas: Vec<_> = dados
        .into_iter()
        .map(|c| {
            // Linha-knob (mockup): largura relativa ao maior grupo; o knob (CSS ::after) marca o fim.
            let largura = (c.quantidade as f64 / maximo as f64 * 100.0).round();
            let cor = cor_status(&c.rotulo); // semáforo §12
            view! {
                <div class="barra-linha">
                    <span class="barra-linha__rotulo">{rotulo_status(&c.rotulo)}</span>
                    <span class="barra-linha__trilho">
                        <span
                            class="barra-linha__preenche"
                            style=format!("width:{largura}%;background:{cor}")
                        ></span>
                    </span>
                    <span class="barra-linha__valor">{fmt_milhar(c.quantidade)}</span>
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
                                view! { <p class="form-auth__erro">{mensagem_usuario(&e)}</p> }.into_any()
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
                                view! { <p class="form-auth__erro">{mensagem_usuario(&e)}</p> }.into_any()
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
    view! {
        <li class="critico-item">
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
