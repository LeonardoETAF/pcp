//! Operação (doc 05 §3/§4): health checks do pipeline/dados + status das execuções recentes.
//! Admin-only — a API responde 403 a não-admin (deny-by-default §7). Frontend burro: exibe o
//! veredito pronto da API (ok/atenção/crítico), sem reavaliar nada (§3).

use leptos::prelude::*;

use crate::api::{admin_pipeline, admin_saude, ExecucaoPipeline, RelatorioSaude, VerificacaoSaude};
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
            <header class="pagina__cab">
                <h1 class="pagina__titulo">"Operação"</h1>
                <p class="texto-suave">
                    "Saúde do pipeline e dados (doc 05 §4) e execuções recentes."
                </p>
            </header>

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
                            view! { <p class="estado-vazio">"Nenhuma execução registrada."</p> }
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

/// Cartões dos health checks (status com cor de semáforo).
fn secao_saude(r: &RelatorioSaude) -> impl IntoView {
    let cartoes: Vec<_> = r.verificacoes.iter().cloned().map(verificacao).collect();
    view! { <div class="saude-grade">{cartoes}</div> }
}

fn verificacao(v: VerificacaoSaude) -> impl IntoView {
    let classe = format!("badge badge--saude-{}", v.status);
    let rotulo = match v.status.as_str() {
        "ok" => "OK",
        "atencao" => "Atenção",
        _ => "Crítico",
    };
    view! {
        <div class="saude-item">
            <div class="saude-item__topo">
                <span class="saude-item__nome">{v.nome}</span>
                <span class=classe>{rotulo}</span>
            </div>
            <p class="saude-item__detalhe">{v.detalhe}</p>
        </div>
    }
}

#[component]
fn TabelaExecucoes(execs: Vec<ExecucaoPipeline>) -> impl IntoView {
    view! {
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
