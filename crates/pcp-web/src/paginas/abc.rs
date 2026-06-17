//! Classificação ABC (doc 03 §6): card de distribuição, gráfico de Pareto (top 20), tabela 1
//! linha por produto (classificação mais recente) com busca e exportação, e workflow de fora de
//! linha (fila de sugestões + aprovação do gestor com auditoria). Frontend burro (§3).

use std::fmt::Write as _;

use leptos::prelude::*;

use crate::api::{
    abc_distribuicao, abc_tabela, listar_ciclo_vida, perfil, transicionar_ciclo_vida,
    DistribuicaoAbc, LinhaAbc, SugestaoCicloVida,
};
use crate::contexto::Sessao;
use crate::download;
use crate::formato::{fmt_milhar, rotulo_status};

#[component]
pub fn ClassificacaoAbc() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let busca = RwSignal::new(String::new());

    let tabela = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => abc_tabela(t).await.unwrap_or_default(),
                None => Vec::new(),
            }
        },
    );
    // Distribuição por classe vem agregada do servidor (§15 — nunca contada no cliente).
    let distribuicao = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => abc_distribuicao(t).await.unwrap_or_default(),
                None => Vec::new(),
            }
        },
    );

    view! {
        <section class="pagina">
            <header class="pagina__cab">
                <h1 class="pagina__titulo">"Classificação ABC"</h1>
                <p class="texto-suave">"Curva de Pareto e consulta da classificação mais recente."</p>
            </header>

            <Suspense fallback=|| view! { <p class="texto-suave">"Carregando classificação…"</p> }>
                {move || {
                    let linhas = tabela.get().unwrap_or_default();
                    let distribuicao = distribuicao.get().unwrap_or_default();
                    if linhas.is_empty() {
                        view! { <p class="estado-vazio">"Sem classificação disponível."</p> }
                            .into_any()
                    } else {
                        view! { <ConteudoAbc linhas distribuicao busca /> }.into_any()
                    }
                }}
            </Suspense>

            <ForaDeLinha />
        </section>
    }
}

/// Card de distribuição + Pareto + tabela com busca/exportação.
#[component]
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // contagens pequenas
fn ConteudoAbc(
    linhas: Vec<LinhaAbc>,
    distribuicao: Vec<DistribuicaoAbc>,
    busca: RwSignal<String>,
) -> impl IntoView {
    let total = linhas.len();
    let dados = StoredValue::new(linhas);

    let top20 = dados.with_value(|l| l.iter().take(20).cloned().collect::<Vec<_>>());

    let exportar = move |_| {
        let csv = dados.with_value(|l| csv_abc(l));
        download::baixar("classificacao-abc.csv", &csv);
    };

    let filtradas = move || {
        let b = busca.get().to_lowercase();
        dados.with_value(|l| {
            l.iter()
                .filter(|x| {
                    b.is_empty()
                        || x.codigo_estoque.to_lowercase().contains(&b)
                        || x.produto
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&b)
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    };

    view! {
        <div class="abc-resumo">
            <div class="kpi">
                <span class="kpi__valor">{fmt_milhar(total as i64)}</span>
                <span class="kpi__rotulo">"Produtos classificados"</span>
            </div>
            <div class="abc-dist">
                {distribuicao
                    .into_iter()
                    .map(|d| {
                        let badge = format!("badge badge--abc-{}", d.classe.to_lowercase());
                        view! {
                            <div class="cob-classe">
                                <span class=badge>{d.classe}</span>
                                <span class="cob-classe__valor">{fmt_milhar(d.quantidade)}</span>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>

        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Pareto — top 20 por volume"</h2>
                <p class="texto-suave">"Barras = volume; linha = % acumulado"</p>
            </header>
            <GraficoPareto dados=top20 />
        </section>

        <div class="filtros-estoque">
            <div class="filtros-estoque__busca">
                <input
                    class="input"
                    placeholder="Buscar por código ou produto…"
                    prop:value=move || busca.get()
                    on:input=move |ev| busca.set(event_target_value(&ev))
                />
            </div>
            <div class="barra-exportar">
                <button type="button" class="btn btn--secundario btn--sm" on:click=exportar>
                    "Exportar CSV"
                </button>
            </div>
        </div>

        <div class="tabela-rolavel">
            <table class="tabela">
                <thead>
                    <tr>
                        <th>"Código"</th>
                        <th>"Produto"</th>
                        <th>"Classe"</th>
                        <th class="tabela__num">"Volume"</th>
                        <th class="tabela__num">"% acum."</th>
                        <th class="tabela__num">"Fator"</th>
                        <th class="tabela__num">"Estoque"</th>
                        <th>"Status"</th>
                    </tr>
                </thead>
                <tbody>
                    {move || {
                        filtradas()
                            .into_iter()
                            .map(|l| view! { <LinhaTabela l /> })
                            .collect_view()
                    }}
                </tbody>
            </table>
        </div>
    }
}

#[component]
fn LinhaTabela(l: LinhaAbc) -> impl IntoView {
    let nome = l
        .produto
        .clone()
        .unwrap_or_else(|| l.codigo_estoque.clone());
    let pct = l
        .percentual_acumulado
        .map_or_else(|| "—".to_owned(), |p| format!("{p:.1}%"));
    view! {
        <tr>
            <td class="tabela__cod">{l.codigo_estoque.clone()}</td>
            <td>{nome}</td>
            <td>
                <span class=format!("badge badge--abc-{}", l.classe.to_lowercase())>
                    {l.classe.clone()}
                </span>
            </td>
            <td class="tabela__num">{fmt_milhar(l.volume_janela)}</td>
            <td class="tabela__num">{pct}</td>
            <td class="tabela__num">{format!("{:.2}", l.fator_estoque)}</td>
            <td class="tabela__num">{fmt_milhar(l.estoque_atual)}</td>
            <td>
                <span class=format!("badge badge--status-{}", l.status)>
                    {rotulo_status(&l.status)}
                </span>
            </td>
        </tr>
    }
}

/// Gráfico de Pareto: barras de volume (top 20) + linha de % acumulado.
#[component]
#[allow(clippy::cast_precision_loss)] // séries curtas (≤20): conversão exata para f64
fn GraficoPareto(dados: Vec<LinhaAbc>) -> impl IntoView {
    const W: f64 = 680.0;
    const H: f64 = 180.0;
    if dados.is_empty() {
        return view! { <p class="estado-vazio">"Sem dados."</p> }.into_any();
    }
    let max = dados
        .iter()
        .map(|d| d.volume_janela)
        .max()
        .unwrap_or(1)
        .max(1) as f64;
    let n = dados.len();
    let largura = W / n as f64;
    let barras: Vec<_> = dados
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let altura = (d.volume_janela as f64 / max) * H;
            let x = i as f64 * largura;
            view! {
                <rect
                    x=format!("{x:.2}")
                    y=format!("{:.2}", H - altura)
                    width=format!("{:.2}", (largura * 0.7).max(1.0))
                    height=format!("{altura:.2}")
                    class="graf__barra"
                />
            }
        })
        .collect();
    let mut linha = String::new();
    for (i, d) in dados.iter().enumerate() {
        let pct = d.percentual_acumulado.unwrap_or(0.0).clamp(0.0, 100.0);
        let x = i as f64 * largura + largura / 2.0;
        let y = H - (pct / 100.0) * H;
        let _ = write!(linha, "{x:.1},{y:.1} ");
    }
    view! {
        <svg class="grafico" viewBox=format!("0 0 {W} {H}") preserveAspectRatio="none">
            {barras} <polyline points=linha class="graf__linha" fill="none" />
        </svg>
    }
    .into_any()
}

/// Monta o CSV da tabela ABC (UTF-8 com BOM, separador `;`, decimais com vírgula — Excel BR).
fn csv_abc(linhas: &[LinhaAbc]) -> String {
    let mut s = String::from('\u{FEFF}');
    s.push_str("Código;Produto;Classe;Volume;% acumulado;Fator;Estoque atual;Status\r\n");
    for l in linhas {
        let produto = l.produto.as_deref().unwrap_or_default();
        let produto = if produto.contains([';', '"', '\n']) {
            format!("\"{}\"", produto.replace('"', "\"\""))
        } else {
            produto.to_owned()
        };
        let pct = l
            .percentual_acumulado
            .map_or_else(String::new, |p| format!("{p:.1}").replace('.', ","));
        let _ = write!(
            s,
            "{};{};{};{};{};{};{};{}\r\n",
            l.codigo_estoque,
            produto,
            l.classe,
            l.volume_janela,
            pct,
            format!("{:.2}", l.fator_estoque).replace('.', ","),
            l.estoque_atual,
            l.status,
        );
    }
    s
}

/// Rótulo da ação de ciclo de vida.
fn rotulo_acao(acao: &str) -> &'static str {
    match acao {
        "sair" => "Sair de linha",
        "voltar" => "Voltar à linha",
        _ => "—",
    }
}

/// Ações do gestor por estado (espelha a máquina do `pcp-core`; o servidor revalida).
fn acoes_estado(estado: &str) -> Vec<(&'static str, &'static str)> {
    match estado {
        "gerada" => vec![("Analisar", "em_analise")],
        "em_analise" => vec![("Aplicar", "aplicada"), ("Recusar", "recusada")],
        _ => vec![],
    }
}

/// Workflow de fora de linha: fila de sugestões abertas + aprovação do gestor (auditada).
#[component]
fn ForaDeLinha() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let recarregar = RwSignal::new(0_u32);
    let msg = RwSignal::new(None::<String>);

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

    let fila = Resource::new(
        move || (sessao.0.get(), recarregar.get()),
        |(t, _)| async move {
            match t {
                Some(t) => listar_ciclo_vida(t).await.unwrap_or_default(),
                None => Vec::new(),
            }
        },
    );

    let transicionar = move |id: String, para: &'static str| {
        let Some(token) = sessao.0.get_untracked() else {
            return;
        };
        leptos::task::spawn_local(async move {
            match transicionar_ciclo_vida(token, id, para.to_owned()).await {
                Ok(_) => recarregar.update(|n| *n += 1),
                Err(e) => msg.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <div>
                    <h2 class="cartao__titulo">"Fora de linha — fila de sugestões"</h2>
                    <p class="texto-suave">"Ciclo de vida do produto; aprovação pelo gestor."</p>
                </div>
            </header>
            {move || msg.get().map(|m| view! { <p class="form-auth__erro">{m}</p> })}
            <Suspense fallback=|| view! { <p class="texto-suave">"Carregando fila…"</p> }>
                {move || {
                    let itens = fila.get().unwrap_or_default();
                    if itens.is_empty() {
                        view! { <p class="estado-vazio">"Nenhuma sugestão aberta."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="solic-lista">
                                {itens
                                    .into_iter()
                                    .map(|s| linha_sugestao(&s, eh_gestor(), transicionar))
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

fn linha_sugestao(
    s: &SugestaoCicloVida,
    eh_gestor: bool,
    transicionar: impl Fn(String, &'static str) + Copy + Send + Sync + 'static,
) -> impl IntoView {
    let acoes = if eh_gestor {
        acoes_estado(&s.estado)
    } else {
        vec![]
    };
    let id = s.id.clone();
    let criterios = s.criterios.join(", ");
    view! {
        <li class="solic-item">
            <span class=format!("badge badge--certeza-{}", s.nivel_certeza)>
                {s.nivel_certeza.clone()}
            </span>
            <div class="solic-item__dados">
                <span class="solic-item__qtd">
                    {format!("{} — {}", s.codigo_estoque, rotulo_acao(&s.acao_sugerida))}
                </span>
                <span class="texto-suave">
                    {format!("Pontuação {} · {}", s.pontuacao, criterios)}
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
