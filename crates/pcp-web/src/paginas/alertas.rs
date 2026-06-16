//! Central de Alertas (doc 03 §5): fila do dia por urgência. Frontend burro (§3): consome
//! `/pcp/alertas` (valores já calculados pelo motor) e só exibe. Tempo real por polling leve
//! (§16, fallback); SSE-com-auth (via cookie) fica como upgrade futuro.

use std::time::Duration;

use leptos::prelude::*;
use leptos_router::components::A;

use crate::api::{alertas, AlertaResumo};
use crate::contexto::Sessao;

/// Nome de exibição "{produto} - {cor}" — cor = texto após ':' da configuração (doc 02 §10).
fn nome_exibicao(a: &AlertaResumo) -> String {
    let produto = a
        .produto
        .clone()
        .unwrap_or_else(|| a.codigo_estoque.clone());
    match a.configuracao.as_deref().and_then(|c| c.split(':').nth(1)) {
        Some(cor) if !cor.trim().is_empty() => format!("{produto} - {}", cor.trim()),
        _ => produto,
    }
}

/// Ruptura iminente: cobertura ≤ 3 dias ou sem estoque (doc 03 §5.1).
fn ruptura_iminente(a: &AlertaResumo) -> bool {
    a.cobertura_dias <= 3.0 || a.status.as_deref() == Some("sem_estoque")
}

fn rotulo_prioridade(p: &str) -> &'static str {
    match p {
        "critico" => "Crítico",
        "alto" => "Alto",
        "medio" => "Médio",
        _ => "—",
    }
}

#[component]
pub fn PaginaAlertas() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let tick = RwSignal::new(0_u32);

    // Polling leve = atualização "em tempo real" do dia a dia (refetch a cada 20 s).
    if let Ok(handle) = set_interval_with_handle(
        move || tick.update(|n| *n = n.wrapping_add(1)),
        Duration::from_secs(20),
    ) {
        on_cleanup(move || handle.clear());
    }

    let dados = Resource::new(
        move || (sessao.0.get(), tick.get()),
        |(token, _)| async move {
            match token {
                Some(t) => alertas(t).await,
                None => Ok(Vec::new()),
            }
        },
    );

    let filtro = RwSignal::new(None::<String>);
    let busca = RwSignal::new(String::new());

    view! {
        <section class="pagina">
            <header class="pagina__cab">
                <h1 class="pagina__titulo">"Central de Alertas"</h1>
                <p class="texto-suave">"Fila do dia — o que produzir, em ordem de urgência."</p>
            </header>
            <Suspense fallback=|| {
                view! { <p class="texto-suave">"Carregando alertas…"</p> }
            }>
                {move || {
                    dados
                        .get()
                        .map(|res| match res {
                            Err(e) => {
                                view! { <p class="form-auth__erro">{e.to_string()}</p> }.into_any()
                            }
                            Ok(lista) => corpo(lista, filtro, busca).into_any(),
                        })
                }}
            </Suspense>
        </section>
    }
}

/// Conteúdo (cards + filtros + lista) para os alertas de uma carga.
fn corpo(
    lista: Vec<AlertaResumo>,
    filtro: RwSignal<Option<String>>,
    busca: RwSignal<String>,
) -> impl IntoView {
    let lista = StoredValue::new(lista);
    let total = lista.with_value(Vec::len);
    let conta = move |p: &str| lista.with_value(|l| l.iter().filter(|a| a.prioridade == p).count());
    let (criticos, altos, medios) = (conta("critico"), conta("alto"), conta("medio"));

    let filtradas = move || {
        let f = filtro.get();
        let b = busca.get().to_lowercase();
        lista.with_value(|l| {
            l.iter()
                .filter(|a| match f.as_deref() {
                    Some(p) => a.prioridade == p,
                    None => true,
                })
                .filter(|a| {
                    b.is_empty()
                        || nome_exibicao(a).to_lowercase().contains(&b)
                        || a.codigo_estoque.to_lowercase().contains(&b)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    };

    view! {
        <div class="cards-resumo">
            <CartaResumo rotulo="Total" valor=total realce="" />
            <CartaResumo rotulo="Críticos" valor=criticos realce="critico" />
            <CartaResumo rotulo="Altos" valor=altos realce="alto" />
            <CartaResumo rotulo="Médios" valor=medios realce="medio" />
        </div>
        <div class="filtros">
            <input
                class="input filtros__busca"
                placeholder="Buscar por produto ou código…"
                prop:value=move || busca.get()
                on:input=move |ev| busca.set(event_target_value(&ev))
            />
            <div class="chips">
                <Chip filtro rotulo="Todos" valor=None />
                <Chip filtro rotulo="Crítico" valor=Some("critico") />
                <Chip filtro rotulo="Alto" valor=Some("alto") />
                <Chip filtro rotulo="Médio" valor=Some("medio") />
            </div>
        </div>
        {move || {
            let itens = filtradas();
            if itens.is_empty() {
                view! { <p class="estado-vazio">"Nenhum alerta para os filtros atuais."</p> }
                    .into_any()
            } else {
                view! {
                    <div class="alertas-lista">
                        {itens.into_iter().map(|a| view! { <LinhaAlerta a /> }).collect_view()}
                    </div>
                }
                    .into_any()
            }
        }}
    }
}

#[component]
fn CartaResumo(rotulo: &'static str, valor: usize, realce: &'static str) -> impl IntoView {
    let classe = if realce.is_empty() {
        "carta-resumo".to_owned()
    } else {
        format!("carta-resumo carta-resumo--{realce}")
    };
    view! {
        <div class=classe>
            <span class="carta-resumo__valor">{valor}</span>
            <span class="carta-resumo__rotulo">{rotulo}</span>
        </div>
    }
}

#[component]
fn Chip(
    filtro: RwSignal<Option<String>>,
    rotulo: &'static str,
    valor: Option<&'static str>,
) -> impl IntoView {
    let ativo = move || filtro.get().as_deref() == valor;
    view! {
        <button
            type="button"
            class="chip"
            class:chip--ativo=ativo
            on:click=move |_| filtro.set(valor.map(ToOwned::to_owned))
        >
            {rotulo}
        </button>
    }
}

#[component]
fn LinhaAlerta(a: AlertaResumo) -> impl IntoView {
    let href = format!("/estoque/{}", a.codigo_estoque);
    let nome = nome_exibicao(&a);
    let ruptura = ruptura_iminente(&a);
    let classe_abc = format!("badge badge--abc-{}", a.classe.to_lowercase());
    let classe_prio = format!("badge badge--prio-{}", a.prioridade);
    view! {
        <article class="linha-alerta">
            <div class="linha-alerta__principal">
                <span class=classe_prio>{rotulo_prioridade(&a.prioridade)}</span>
                <div class="linha-alerta__nome">
                    <span class="linha-alerta__produto">{nome}</span>
                    <span class="linha-alerta__codigo">{a.codigo_estoque.clone()}</span>
                </div>
                {ruptura
                    .then(|| view! { <span class="pill-ruptura">"Ruptura iminente"</span> })}
            </div>
            <div class="linha-alerta__metricas">
                <span class=classe_abc>{a.classe.clone()}</span>
                <Metrica rotulo="Cobertura" valor=format!("{:.1} d", a.cobertura_dias) />
                <Metrica rotulo="Sugerido" valor=a.qtd_sugerida.to_string() />
                <A href=href attr:class="btn btn--secundario linha-alerta__link">
                    "Detalhes"
                </A>
            </div>
        </article>
    }
}

#[component]
fn Metrica(rotulo: &'static str, valor: String) -> impl IntoView {
    view! {
        <span class="metrica">
            <span class="metrica__rotulo">{rotulo}</span>
            <span class="metrica__valor">{valor}</span>
        </span>
    }
}
