//! Detalhe do Produto (doc 03 §4): cabeçalho, métricas e — nas próximas seções — status/histórico
//! de produção e movimentação. Frontend burro (§3): tudo vem pronto da API.

use leptos::prelude::*;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::api::{produto_detalhe, DetalheProduto, MetricasProduto};
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
        <header class="prod-cab">
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
        </header>

        {metricas(&d.metricas)}
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
