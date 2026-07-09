//! Detalhe do Produto (doc 03 §4): cabeçalho com regra da classe, métricas e gráficos de 90 dias
//! (vendas e estoque), renderizados em SVG (Rust). Frontend burro (§3): tudo vem pronto da API.
//! Insights de IA e Solicitação de Produção entram no próximo passo (2.5 parte 2 / Fase 4).

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::api::{
    criar_solicitacao, listar_solicitacoes, perfil, produto_detalhe, produto_insights,
    transicionar_solicitacao, AlertaInteligente, DetalheProduto, Insights, MetricasProduto, Ponto,
    Recomendacao, RegraClasse, Solicitacao,
};
use crate::componentes::EstadoVazio;
use crate::componentes::Seletor;
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

        <InsightsSecao codigo=d.codigo_estoque.clone() />
        <CentroComando codigo=d.codigo_estoque.clone() recomendacao=d.recomendacao.clone() />
    }
}

fn rotulo_severidade(s: &str) -> &'static str {
    match s {
        "critico" => "Crítico",
        "atencao" => "Atenção",
        "positivo" => "Positivo",
        _ => "Info",
    }
}

/// Insights inteligentes (doc 03 §4.1 / doc 06 §3): alertas, previsão e tendência — calculados no
/// backend (`pcp-ai`). Frontend burro: só exibe.
#[component]
fn InsightsSecao(codigo: String) -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let codigo = StoredValue::new(codigo);
    let dados = Resource::new(
        move || sessao.0.get(),
        move |t| async move {
            match t {
                Some(t) => produto_insights(t, codigo.get_value()).await.ok(),
                None => None,
            }
        },
    );
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Insights inteligentes"</h2>
                <p class="texto-suave">"Tendência, previsão e alertas (motor estatístico)."</p>
            </header>
            <Suspense fallback=|| view! { <p class="texto-suave">"Analisando…"</p> }>
                {move || {
                    dados
                        .get()
                        .flatten()
                        .map_or_else(
                            || {
                                view! {
                                    <p class="estado-vazio">"Sem dados suficientes para insights."</p>
                                }
                                    .into_any()
                            },
                            |i| corpo_insights(&i).into_any(),
                        )
                }}
            </Suspense>
        </section>
    }
}

#[allow(clippy::cast_possible_truncation)] // previsões pequenas: arredondamento p/ exibição
fn corpo_insights(i: &Insights) -> impl IntoView {
    let tendencia = if i.slope > 0.05 {
        "Alta"
    } else if i.slope < -0.05 {
        "Queda"
    } else {
        "Estável"
    };
    let alertas = i.alertas.clone();
    view! {
        <div class="prod-metricas">
            <div class="metrica-card">
                <span class="metrica-card__rotulo">"Previsão 7 dias"</span>
                <span class="metrica-card__valor">{fmt_milhar(i.total_previsto_7d.round() as i64)}</span>
            </div>
            <div class="metrica-card">
                <span class="metrica-card__rotulo">"Previsão 30 dias"</span>
                <span class="metrica-card__valor">
                    {fmt_milhar(i.total_previsto_30d.round() as i64)}
                </span>
            </div>
            <div class="metrica-card">
                <span class="metrica-card__rotulo">"Confiança"</span>
                <span class="metrica-card__valor">{format!("{:.0}%", i.confianca * 100.0)}</span>
            </div>
            <div class="metrica-card">
                <span class="metrica-card__rotulo">"Tendência"</span>
                <span class="metrica-card__valor">{tendencia}</span>
            </div>
            <div class="metrica-card">
                <span class="metrica-card__rotulo">"Força sazonal"</span>
                <span class="metrica-card__valor">{format!("{:.2}", i.forca_sazonal)}</span>
            </div>
            <div class="metrica-card">
                <span class="metrica-card__rotulo">"Dias com venda"</span>
                <span class="metrica-card__valor">{format!("{:.0}%", i.dias_com_venda_pct)}</span>
            </div>
        </div>
        {(!alertas.is_empty())
            .then(|| {
                view! {
                    <ul class="solic-lista insights-alertas">
                        {alertas.into_iter().map(|a| linha_alerta_ia(&a)).collect_view()}
                    </ul>
                }
            })}
    }
}

fn linha_alerta_ia(a: &AlertaInteligente) -> impl IntoView {
    let badge = format!("badge badge--sev-{}", a.severidade);
    view! {
        <li class="solic-item">
            <span class=badge>{rotulo_severidade(&a.severidade)}</span>
            <div class="solic-item__dados">
                <span class="solic-item__qtd">{a.titulo.clone()}</span>
                <span class="texto-suave">{a.detalhe.clone()}</span>
            </div>
        </li>
    }
}

/// Rótulo pt-BR do estado da solicitação (doc 03 §4.3).
fn rotulo_estado(estado: &str) -> &'static str {
    match estado {
        "pendente" => "Pendente",
        "aprovada" => "Aprovada",
        "em_producao" => "Em produção",
        "concluida" => "Concluída",
        "recusada" => "Recusada",
        _ => "—",
    }
}

/// Ações disponíveis (rótulo, estado destino) a partir do estado atual — espelha a máquina de
/// estados do `pcp-core` (só habilita transições válidas; o servidor revalida).
fn acoes_estado(estado: &str) -> Vec<(&'static str, &'static str)> {
    match estado {
        "pendente" => vec![("Aprovar", "aprovada"), ("Recusar", "recusada")],
        "aprovada" => vec![("Em produção", "em_producao")],
        "em_producao" => vec![("Concluir", "concluida")],
        _ => vec![],
    }
}

/// Centro de comando (doc 03 §4.1): gerar Solicitação de Produção (default da recomendação,
/// editável) e acompanhar/avançar o status. Frontend burro: a regra/validação é do servidor.
#[component]
#[allow(clippy::too_many_lines)] // markup do formulário + lista
fn CentroComando(codigo: String, recomendacao: Recomendacao) -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let codigo = StoredValue::new(codigo);
    let recarregar = RwSignal::new(0_u32);
    let qtd = RwSignal::new(recomendacao.qtd_sugerida.to_string());
    let prioridade = RwSignal::new(recomendacao.prioridade.clone());
    let justificativa = RwSignal::new(String::new());
    let msg = RwSignal::new(None::<String>);
    let auto = recomendacao.aprovacao_automatica;

    let papel = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => perfil(t).await.unwrap_or_default(),
                None => String::new(),
            }
        },
    );
    let eh_gestor = move || matches!(papel.get().as_deref(), Some("gestor" | "admin"));

    let lista = Resource::new(
        move || (sessao.0.get(), recarregar.get()),
        move |(t, _)| async move {
            match t {
                Some(t) => listar_solicitacoes(t, codigo.get_value())
                    .await
                    .unwrap_or_default(),
                None => Vec::new(),
            }
        },
    );

    let gerar = move |_| {
        let Some(token) = sessao.0.get_untracked() else {
            return;
        };
        let qtd_val = qtd.get_untracked().trim().parse::<i64>().unwrap_or(0);
        if qtd_val <= 0 {
            msg.set(Some("Informe uma quantidade positiva.".to_owned()));
            return;
        }
        let prio = prioridade.get_untracked();
        let just = justificativa.get_untracked();
        leptos::task::spawn_local(async move {
            match criar_solicitacao(token, codigo.get_value(), qtd_val, prio, just).await {
                Ok(_) => {
                    justificativa.set(String::new());
                    msg.set(None);
                    recarregar.update(|n| *n += 1);
                }
                Err(e) => msg.set(Some(e.to_string())),
            }
        });
    };

    let transicionar = move |id: String, para: &'static str| {
        let Some(token) = sessao.0.get_untracked() else {
            return;
        };
        leptos::task::spawn_local(async move {
            match transicionar_solicitacao(token, id, para.to_owned()).await {
                Ok(_) => recarregar.update(|n| *n += 1),
                Err(e) => msg.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <section class="cartao centro-comando">
            <header class="cartao__cab">
                <div>
                    <h2 class="cartao__titulo">"Centro de comando"</h2>
                    <p class="texto-suave">"Gerar solicitação de produção e acompanhar o status."</p>
                </div>
            </header>

            <div class="solic-form">
                <label class="campo-select">
                    <span class="campo-select__rotulo">"Quantidade a produzir"</span>
                    <input
                        class="input input--num"
                        type="number"
                        min="1"
                        prop:value=move || qtd.get()
                        on:input=move |ev| qtd.set(event_target_value(&ev))
                    />
                </label>
                <div class="campo-select">
                    <span class="campo-select__rotulo">"Prioridade"</span>
                    <Seletor
                        rotulo="Prioridade"
                        opcoes=vec![("alta", "Alta"), ("media", "Média"), ("baixa", "Baixa")]
                        valor=Signal::derive(move || prioridade.get())
                        ao_escolher=move |v| prioridade.set(v)
                    />
                </div>
                <label class="campo-select solic-form__just">
                    <span class="campo-select__rotulo">"Justificativa"</span>
                    <input
                        class="input"
                        placeholder="Opcional"
                        prop:value=move || justificativa.get()
                        on:input=move |ev| justificativa.set(event_target_value(&ev))
                    />
                </label>
                <button type="button" class="btn btn--primario" on:click=gerar>
                    "Gerar solicitação"
                </button>
            </div>
            {auto
                .then(|| {
                    view! {
                        <p class="texto-suave solic-auto">
                            "Dentro do limite de aprovação automática (qtd e prioridade) — entra já como aprovada."
                        </p>
                    }
                })}
            {move || {
                msg.get().map(|m| view! { <p class="form-auth__erro">{m}</p> })
            }}

            <Suspense fallback=|| view! { <p class="texto-suave">"Carregando solicitações…"</p> }>
                {move || {
                    lista
                        .get()
                        .map(|itens| {
                            if itens.is_empty() {
                                view! {
                                    <EstadoVazio
                                        arte="empty-orders.svg"
                                        titulo="Nenhuma solicitação para este produto"
                                    />
                                }
                                    .into_any()
                            } else {
                                view! {
                                    <ul class="solic-lista">
                                        {itens
                                            .into_iter()
                                            .map(|s| linha_solicitacao(&s, eh_gestor(), transicionar))
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

fn linha_solicitacao(
    s: &Solicitacao,
    eh_gestor: bool,
    transicionar: impl Fn(String, &'static str) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let classe_estado = format!("badge badge--sol-{}", s.estado);
    let acoes = if eh_gestor {
        acoes_estado(&s.estado)
    } else {
        vec![]
    };
    let id = s.id.clone();
    view! {
        <li class="solic-item">
            <span class=classe_estado>{rotulo_estado(&s.estado)}</span>
            <div class="solic-item__dados">
                <span class="solic-item__qtd">{fmt_milhar(s.qtd_solicitada)}" un"</span>
                <span class="texto-suave">
                    {format!("Prioridade {} · prazo {}", s.prioridade, s.prazo)}
                </span>
            </div>
            <div class="solic-item__acoes">
                {acoes
                    .into_iter()
                    .map(|(rotulo, destino)| {
                        let id = id.clone();
                        view! {
                            <button
                                type="button"
                                class="btn btn--secundario btn--sm"
                                on:click=move |_| transicionar(id.clone(), destino)
                            >
                                {rotulo}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
        </li>
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
