//! Operação (doc 05 §3/§4): health checks do pipeline/dados + status das execuções recentes.
//! Admin-only — a API responde 403 a não-admin (deny-by-default §7). Frontend burro: exibe o
//! veredito pronto da API (ok/atenção/crítico), sem reavaliar nada (§3).

use leptos::prelude::*;

use crate::api::{
    admin_pipeline, admin_reprocessar, admin_saude, ExecucaoPipeline, RelatorioSaude,
    VerificacaoSaude,
};
use crate::componentes::EstadoVazio;
use crate::contexto::Sessao;
use crate::formato::fmt_milhar;

#[component]
pub fn Operacao() -> impl IntoView {
    let sessao = expect_context::<Sessao>();

    let saude = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => admin_saude(t).await,
                None => Ok(RelatorioSaude::default()),
            }
        },
    );
    let pipeline = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => admin_pipeline(t).await.unwrap_or_default(),
                None => Vec::new(),
            }
        },
    );

    view! {
        <section class="pagina">
            <Suspense fallback=|| {
                view! { <p class="texto-suave">"Carregando saúde…"</p> }
            }>
                {move || {
                    saude
                        .get()
                        .map(|res| match res {
                            Ok(r) => secao_saude(&r).into_any(),
                            Err(_) => {
                                view! {
                                    <p class="estado-vazio">
                                        "Acesso restrito a administradores."
                                    </p>
                                }
                                    .into_any()
                            }
                        })
                }}
            </Suspense>

            <FormReprocesso />

            <section class="cartao">
                <header class="cartao__cab">
                    <h2 class="cartao__titulo">"Execuções do pipeline"</h2>
                    <p class="texto-suave">"Telemetria por módulo (mais recentes primeiro)."</p>
                </header>
                <Suspense fallback=|| {
                    view! { <p class="texto-suave">"Carregando execuções…"</p> }
                }>
                    {move || {
                        let execs = pipeline.get().unwrap_or_default();
                        if execs.is_empty() {
                            view! {
                                <EstadoVazio
                                    arte="empty-movements.svg"
                                    titulo="Nenhuma execução registrada"
                                    descricao="As execuções do pipeline aparecem aqui assim que a primeira rodar."
                                />
                            }
                                .into_any()
                        } else {
                            view! { <TabelaExecucoes execs /> }.into_any()
                        }
                    }}
                </Suspense>
            </section>
        </section>
    }
}

/// Formulário de reprocesso de intervalo (admin). Dispara `POST /pcp/admin/reprocessar`; o
/// resultado real aparece na tabela de execuções e nos health checks. Não admin → erro da API.
#[component]
fn FormReprocesso() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let inicio = RwSignal::new(String::new());
    let fim = RwSignal::new(String::new());
    let msg = RwSignal::new(String::new());

    let enviar = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        let Some(token) = sessao.0.get_untracked() else {
            return;
        };
        let (i, f) = (inicio.get_untracked(), fim.get_untracked());
        if i.is_empty() || f.is_empty() {
            msg.set("Informe a data inicial e a final.".to_owned());
            return;
        }
        msg.set("Enviando…".to_owned());
        leptos::task::spawn_local(async move {
            match admin_reprocessar(token, i, f).await {
                Ok(m) => msg.set(m),
                Err(e) => msg.set(e.to_string()),
            }
        });
    };

    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Reprocessar"</h2>
                <p class="texto-suave">
                    "Recalcula o pipeline de um intervalo (idempotente). Exige os dados do período."
                </p>
            </header>
            <form class="form-reprocesso" on:submit=enviar>
                <label class="campo-select">
                    <span class="campo-select__rotulo">"Início"</span>
                    <input
                        class="input"
                        type="date"
                        prop:value=move || inicio.get()
                        on:input=move |ev| inicio.set(event_target_value(&ev))
                    />
                </label>
                <label class="campo-select">
                    <span class="campo-select__rotulo">"Fim"</span>
                    <input
                        class="input"
                        type="date"
                        prop:value=move || fim.get()
                        on:input=move |ev| fim.set(event_target_value(&ev))
                    />
                </label>
                <button type="submit" class="btn btn--secundario">
                    "Reprocessar"
                </button>
            </form>
            {move || {
                let m = msg.get();
                (!m.is_empty()).then(|| view! { <p class="texto-suave">{m}</p> })
            }}
        </section>
    }
}

/// Cartões dos health checks (status com cor de semáforo).
fn secao_saude(r: &RelatorioSaude) -> impl IntoView {
    let cartoes: Vec<_> = r.verificacoes.iter().cloned().map(verificacao).collect();
    view! { <div class="saude-grade">{cartoes}</div> }
}

/// Ícone do card de saúde: crítico/atenção → alerta; OK → ícone próprio da verificação (nome).
fn icone_saude(nome: &str, status: &str) -> &'static str {
    if status != "ok" {
        return "alerta.svg";
    }
    if nome.starts_with("Duração") || nome.starts_with("Snapshot") {
        "relogio.svg"
    } else if nome.starts_with("Geração de alertas") {
        "notificacao.svg"
    } else if nome.starts_with("CV") || nome.starts_with("Última execução") {
        "atividade.svg"
    } else {
        "confirmar.svg"
    }
}

fn verificacao(v: VerificacaoSaude) -> impl IntoView {
    let classe = format!("saude-item saude-item--{}", v.status);
    let badge = format!("badge badge--saude-{}", v.status);
    let rotulo = match v.status.as_str() {
        "ok" => "OK",
        "atencao" => "Atenção",
        _ => "Crítico",
    };
    let icone = icone_saude(&v.nome, &v.status);
    let estilo = format!("-webkit-mask-image:url(/icons/{icone});mask-image:url(/icons/{icone})");
    view! {
        <div class=classe>
            <div class="saude-item__topo">
                <span class="saude-item__chip">
                    <span class="icone-mask" style=estilo></span>
                </span>
                <span class="saude-item__nome">{v.nome}</span>
            </div>
            <p class="saude-item__detalhe">{v.detalhe}</p>
            <span class=badge>{rotulo}</span>
        </div>
    }
}

#[component]
fn TabelaExecucoes(execs: Vec<ExecucaoPipeline>) -> impl IntoView {
    view! {
        <div class="tabela-cartao">
            <div class="tabela-rolavel">
                <table class="tabela">
                    <thead>
                        <tr>
                            <th>"Data ref."</th>
                            <th>"Módulo"</th>
                            <th>"Status"</th>
                            <th class="tabela__num">"Linhas"</th>
                            <th class="tabela__num">"Duração"</th>
                            <th>"Erro"</th>
                        </tr>
                    </thead>
                    <tbody>
                        {execs.into_iter().map(linha_execucao).collect_view()}
                    </tbody>
                </table>
            </div>
        </div>
    }
}

fn linha_execucao(e: ExecucaoPipeline) -> impl IntoView {
    let classe = if e.status == "erro" {
        "badge badge--saude-critico"
    } else {
        "badge badge--saude-ok"
    };
    view! {
        <tr>
            <td class="tabela__cod">{e.data_ref}</td>
            <td>{e.modulo}</td>
            <td>
                <span class=classe>{e.status}</span>
            </td>
            <td class="tabela__num">{fmt_milhar(e.linhas_afetadas)}</td>
            <td class="tabela__num">{format!("{} ms", fmt_milhar(e.duracao_ms))}</td>
            <td class="texto-suave">{e.erro.unwrap_or_default()}</td>
        </tr>
    }
}
