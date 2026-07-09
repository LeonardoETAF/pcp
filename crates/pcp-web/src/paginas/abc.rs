//! Classificação ABC (doc 03 §6): card de distribuição, gráfico de Pareto (top 20), tabela 1
//! linha por produto (classificação mais recente) com busca e exportação, e workflow de fora de
//! linha (fila de sugestões + aprovação do gestor com auditoria). Frontend burro (§3).

use std::cmp::Reverse;
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

/// Cor fixa por classe ABC (§12).
fn cor_classe_abc(classe: &str) -> &'static str {
    match classe {
        "A" => "var(--abc-a)",
        "B" => "var(--abc-b)",
        "C" => "var(--abc-c)",
        "D" => "var(--abc-d)",
        "F" => "var(--abc-f)",
        _ => "var(--abc-n)",
    }
}

/// Cor do semáforo de status (§12) para o texto de status.
fn cor_status_abc(status: &str) -> &'static str {
    match status {
        "critico" | "sem_estoque" => "var(--semaforo-critico)",
        "estoque_baixo" => "var(--semaforo-alto)",
        "baixo" => "var(--semaforo-medio)",
        "adequado" => "var(--semaforo-ok)",
        "alto" | "excessivo" => "var(--semaforo-info)",
        _ => "var(--abc-d)",
    }
}

/// Ordem canônica das classes (A→N) para as abas.
fn ordem_classe_abc(c: &str) -> u8 {
    match c {
        "A" => 0,
        "B" => 1,
        "C" => 2,
        "D" => 3,
        "F" => 4,
        "N" => 5,
        _ => 9,
    }
}

/// KPIs + Pareto + abas + tabela com busca/filtros/exportação.
#[component]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::too_many_lines
)]
fn ConteudoAbc(
    linhas: Vec<LinhaAbc>,
    distribuicao: Vec<DistribuicaoAbc>,
    busca: RwSignal<String>,
) -> impl IntoView {
    let total = linhas.len() as i64;
    // KPIs (dado real, frontend burro §3): maior grupo, classe A (produtos-chave), maior volume.
    let maior_grupo = distribuicao
        .iter()
        .max_by_key(|d| d.quantidade)
        .map(|d| (d.classe.clone(), d.quantidade));
    let classe_a = distribuicao
        .iter()
        .find(|d| d.classe == "A")
        .map_or(0, |d| d.quantidade);
    let maior_volume = linhas.iter().map(|l| l.volume_janela).max().unwrap_or(0);

    let mut abas: Vec<(String, i64)> = distribuicao
        .iter()
        .map(|d| (d.classe.clone(), d.quantidade))
        .collect();
    abas.sort_by_key(|(c, _)| ordem_classe_abc(c));

    let dados = StoredValue::new(linhas);
    let top20 = dados.with_value(|l| l.iter().take(20).cloned().collect::<Vec<_>>());

    let classe_filtro = RwSignal::new(None::<String>);
    let status_filtro = RwSignal::new(None::<String>);
    let ordem = RwSignal::new("volume_desc".to_owned());

    let exportar = move |_| {
        let csv = dados.with_value(|l| csv_abc(l));
        download::baixar("classificacao-abc.csv", &csv);
    };

    let filtradas = move || {
        let b = busca.get().to_lowercase();
        let cf = classe_filtro.get();
        let sf = status_filtro.get();
        let mut v = dados.with_value(|l| {
            l.iter()
                .filter(|x| {
                    (b.is_empty()
                        || x.codigo_estoque.to_lowercase().contains(&b)
                        || x.produto
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&b))
                        && cf.as_ref().is_none_or(|c| &x.classe == c)
                        && sf.as_ref().is_none_or(|s| &x.status == s)
                })
                .cloned()
                .collect::<Vec<_>>()
        });
        match ordem.get().as_str() {
            "volume_asc" => v.sort_by_key(|x| x.volume_janela),
            "estoque_desc" => v.sort_by_key(|x| Reverse(x.estoque_atual)),
            "estoque_asc" => v.sort_by_key(|x| x.estoque_atual),
            "produto_asc" => v.sort_by(|a, b| {
                a.produto
                    .as_deref()
                    .unwrap_or("")
                    .cmp(b.produto.as_deref().unwrap_or(""))
            }),
            _ => v.sort_by_key(|x| Reverse(x.volume_janela)),
        }
        v
    };

    let estilo_export =
        "-webkit-mask-image:url(/icons/exportar.svg);mask-image:url(/icons/exportar.svg)";

    view! {
        <div class="kpis">
            <KpiAbc
                icone="inventory.svg"
                valor=fmt_milhar(total)
                rotulo="Produtos classificados".to_owned()
            />
            {maior_grupo
                .map(|(c, q)| {
                    view! {
                        <KpiAbc
                            icone="grafico-barras-h.svg"
                            valor=fmt_milhar(q)
                            rotulo=format!("Classe {c} — maior grupo")
                        />
                    }
                })}
            <KpiAbc
                icone="alerta.svg"
                valor=fmt_milhar(classe_a)
                rotulo="Classe A — produtos-chave".to_owned()
                cor_valor="var(--abc-a)"
            />
            <KpiAbc
                icone="atividade.svg"
                valor=fmt_milhar(maior_volume)
                rotulo="Maior volume individual".to_owned()
            />
        </div>

        <div class="abas-classe">
            <AbaAbc classe_filtro rotulo="Todos".to_owned() valor=None contagem=total />
            {abas
                .into_iter()
                .map(|(cod, qtd)| {
                    view! {
                        <AbaAbc
                            classe_filtro
                            rotulo=cod.clone()
                            valor=Some(cod)
                            contagem=qtd
                        />
                    }
                })
                .collect_view()}
        </div>

        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Pareto — top 20 por volume"</h2>
                <p class="texto-suave">"Barras = volume; linha = % acumulado"</p>
            </header>
            <GraficoPareto dados=top20 />
        </section>

        <div class="estoque-filtros">
            <div class="estoque-filtros__linha">
                <div class="estoque-filtros__busca">
                    <input
                        class="input"
                        placeholder="Buscar por código ou produto…"
                        prop:value=move || busca.get()
                        on:input=move |ev| busca.set(event_target_value(&ev))
                    />
                </div>
                <select
                    class="select"
                    aria-label="Ordenar"
                    on:change=move |ev| ordem.set(event_target_value(&ev))
                    prop:value=move || ordem.get()
                >
                    <option value="volume_desc">"Volume (maior)"</option>
                    <option value="volume_asc">"Volume (menor)"</option>
                    <option value="estoque_desc">"Estoque (maior)"</option>
                    <option value="estoque_asc">"Estoque (menor)"</option>
                    <option value="produto_asc">"Produto (A–Z)"</option>
                </select>
                <select
                    class="select"
                    aria-label="Status"
                    on:change=move |ev| {
                        let v = event_target_value(&ev);
                        status_filtro.set((!v.is_empty()).then_some(v));
                    }
                    prop:value=move || status_filtro.get().unwrap_or_default()
                >
                    <option value="">"Todos os status"</option>
                    <option value="critico">"Crítico"</option>
                    <option value="sem_estoque">"Sem estoque"</option>
                    <option value="estoque_baixo">"Estoque baixo"</option>
                    <option value="baixo">"Baixo"</option>
                    <option value="adequado">"Adequado"</option>
                    <option value="alto">"Alto"</option>
                    <option value="excessivo">"Excessivo"</option>
                    <option value="sem_historico">"Sem histórico"</option>
                    <option value="fora_de_linha">"Fora de Linha"</option>
                </select>
                <button
                    type="button"
                    class="btn-icone"
                    aria-label="Exportar CSV"
                    on:click=exportar
                >
                    <span class="icone-mask" style=estilo_export></span>
                </button>
            </div>
        </div>

        <div class="tabela-cartao">
            <div class="tabela-rolavel">
                <table class="tabela">
                    <thead>
                        <tr>
                            <th>"Código"</th>
                            <th>"Produto"</th>
                            <th>"Cl."</th>
                            <th class="tabela__num">"Volume"</th>
                            <th class="tabela__num">"% acum."</th>
                            <th class="tabela__num">"Fator"</th>
                            <th class="tabela__num">"Estoque"</th>
                            <th class="tabela__status">"Status"</th>
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
        </div>
    }
}

/// KPI horizontal do ABC: chip de ícone à esquerda, número e rótulo (caixa-alta) à direita.
#[component]
fn KpiAbc(
    icone: &'static str,
    valor: String,
    rotulo: String,
    #[prop(optional)] cor_valor: &'static str,
) -> impl IntoView {
    let estilo = format!("-webkit-mask-image:url(/icons/{icone});mask-image:url(/icons/{icone})");
    let estilo_valor = (!cor_valor.is_empty()).then(|| format!("color:{cor_valor}"));
    view! {
        <div class="kpi kpi--linha">
            <span class="kpi__chip">
                <span class="icone-mask" style=estilo></span>
            </span>
            <div class="kpi__corpo">
                <span class="kpi__valor" style=estilo_valor>
                    {valor}
                </span>
                <span class="kpi__rotulo">{rotulo}</span>
            </div>
        </div>
    }
}

/// Aba de filtro por classe (círculo colorido §12 + letra + contagem). Filtra a tabela.
#[component]
fn AbaAbc(
    classe_filtro: RwSignal<Option<String>>,
    rotulo: String,
    valor: Option<String>,
    contagem: i64,
) -> impl IntoView {
    let alvo = valor.clone();
    let ativo = {
        let alvo = alvo.clone();
        move || classe_filtro.get() == alvo
    };
    let ponto = valor
        .as_deref()
        .map(|c| format!("background:{}", cor_classe_abc(c)));
    view! {
        <button
            type="button"
            class="aba"
            class:aba--ativa=ativo
            on:click=move |_| classe_filtro.set(alvo.clone())
        >
            {ponto.map(|p| view! { <span class="aba__pt" style=p></span> })}
            <span>{rotulo}</span>
            <span class="aba__contagem">{fmt_milhar(contagem)}</span>
        </button>
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
        .map_or_else(|| "—".to_owned(), |p| format!("{p:.1}%").replace('.', ","));
    let cor_st = cor_status_abc(&l.status);
    let badge_cl = format!(
        "badge badge--circulo badge--abc-{}",
        l.classe.to_lowercase()
    );
    view! {
        <tr>
            <td class="tabela__cod">{l.codigo_estoque.clone()}</td>
            <td>{nome}</td>
            <td>
                <span class=badge_cl>{l.classe.clone()}</span>
            </td>
            <td class="tabela__num tabela__produzir">{fmt_milhar(l.volume_janela)}</td>
            <td class="tabela__num texto-suave">{pct}</td>
            <td class="tabela__num texto-suave">
                {format!("{:.2}", l.fator_estoque).replace('.', ",")}
            </td>
            <td class="tabela__num">{fmt_milhar(l.estoque_atual)}</td>
            <td class="tabela__status">
                <span class="status-texto" style=format!("color:{cor_st}")>
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
                    <h2 class="cartao__titulo">"Fora de Linha — fila de sugestões"</h2>
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
