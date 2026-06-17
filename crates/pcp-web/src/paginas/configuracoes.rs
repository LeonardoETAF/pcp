//! Configurações (doc 03 §8): edição de TODAS as constantes de negócio (doc 02 §11) com
//! validação no servidor, persistência e recarga a quente, mais a trilha de auditoria (§7.5).
//! Frontend burro (§3): trata a config como JSON opaco — não conhece a regra; só edita valores.

use std::collections::BTreeMap;

use leptos::prelude::*;
use serde_json::Value;

use crate::api::{auditoria_config, obter_config, perfil, salvar_config, EntradaAuditoriaConfig};
use crate::contexto::Sessao;

#[component]
pub fn Configuracoes() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let recarregar = RwSignal::new(0_u32);

    let papel = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => perfil(t).await.unwrap_or_default(),
                None => String::new(),
            }
        },
    );
    let config = Resource::new(
        move || (sessao.0.get(), recarregar.get()),
        |(t, _)| async move {
            match t {
                Some(t) => obter_config(t).await.ok(),
                None => None,
            }
        },
    );

    view! {
        <section class="pagina">
            <header class="pagina__cab">
                <h1 class="pagina__titulo">"Configurações"</h1>
                <p class="texto-suave">
                    "Constantes de negócio (doc 02 §11). Edição restrita ao gestor; toda mudança é auditada."
                </p>
            </header>
            <Suspense fallback=|| view! { <p class="texto-suave">"Carregando configuração…"</p> }>
                {move || {
                    let eh_gestor = matches!(papel.get().as_deref(), Some("gestor" | "admin"));
                    config
                        .get()
                        .flatten()
                        .map(|c| view! { <EditorConfig config=c eh_gestor recarregar /> })
                }}
            </Suspense>
            <AuditoriaConfig />
        </section>
    }
}

#[component]
#[allow(clippy::too_many_lines)] // editor genérico + markup
fn EditorConfig(config: Value, eh_gestor: bool, recarregar: RwSignal<u32>) -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let mut leaves = Vec::new();
    achatar("", &config, &mut leaves);
    let original = StoredValue::new(config);
    let leaves = StoredValue::new(leaves);
    let edits = RwSignal::new(leaves.with_value(|l| {
        l.iter()
            .map(|(p, v)| (p.clone(), valor_para_input(v)))
            .collect::<BTreeMap<String, String>>()
    }));
    let msg = RwSignal::new(None::<(bool, String)>); // (sucesso?, texto)

    let salvar = move |_| {
        let Some(token) = sessao.0.get_untracked() else {
            return;
        };
        let novo =
            leaves.with_value(|l| reconstruir(&original.get_value(), l, &edits.get_untracked()));
        leptos::task::spawn_local(async move {
            match salvar_config(token, novo).await {
                Ok(_) => {
                    msg.set(Some((true, "Configuração salva e recarregada.".to_owned())));
                    recarregar.update(|n| *n += 1);
                }
                Err(e) => msg.set(Some((false, e.to_string()))),
            }
        });
    };

    // Agrupa as folhas por seção (primeiro segmento do caminho).
    let secoes: Vec<(String, Vec<(String, Value)>)> = leaves.with_value(|l| {
        let mut mapa: BTreeMap<String, Vec<(String, Value)>> = BTreeMap::new();
        for (p, v) in l {
            let secao = p.split('.').next().unwrap_or("").to_owned();
            mapa.entry(secao).or_default().push((p.clone(), v.clone()));
        }
        mapa.into_iter().collect()
    });

    view! {
        {move || {
            msg.get()
                .map(|(ok, t)| {
                    let classe = if ok { "form-auth__ok" } else { "form-auth__erro" };
                    view! { <p class=classe>{t}</p> }
                })
        }}
        <div class="config-grade">
            {secoes
                .into_iter()
                .map(|(secao, campos)| {
                    view! {
                        <section class="cartao">
                            <header class="cartao__cab">
                                <h2 class="cartao__titulo">{secao}</h2>
                            </header>
                            <div class="config-campos">
                                {campos
                                    .into_iter()
                                    .map(|(path, orig)| campo(&path, &orig, edits, eh_gestor))
                                    .collect_view()}
                            </div>
                        </section>
                    }
                })
                .collect_view()}
        </div>
        {eh_gestor
            .then(|| {
                view! {
                    <div class="barra-exportar">
                        <button type="button" class="btn btn--primario" on:click=salvar>
                            "Salvar configuração"
                        </button>
                    </div>
                }
            })}
        {(!eh_gestor)
            .then(|| {
                view! {
                    <p class="texto-suave">"Somente leitura — apenas o gestor edita a configuração."</p>
                }
            })}
    }
}

/// Renderiza um campo editável conforme o tipo da folha (bool → checkbox; resto → texto/número).
fn campo(
    path: &str,
    orig: &Value,
    edits: RwSignal<BTreeMap<String, String>>,
    eh_gestor: bool,
) -> impl IntoView {
    let rotulo = path.rsplit('.').next().unwrap_or(path).to_owned();
    let p1 = path.to_owned();
    let valor = move || edits.get().get(&p1).cloned().unwrap_or_default();
    let p2 = path.to_owned();
    let eh_bool = orig.is_boolean();
    let eh_num = orig.is_number();
    let entrada = if eh_bool {
        let p = path.to_owned();
        view! {
            <input
                type="checkbox"
                class="switch"
                prop:disabled=!eh_gestor
                prop:checked=move || edits.get().get(&p2).is_some_and(|s| s == "true")
                on:change=move |ev| {
                    let v = if event_target_checked(&ev) { "true" } else { "false" };
                    edits.update(|m| {
                        m.insert(p.clone(), v.to_owned());
                    });
                }
            />
        }
        .into_any()
    } else {
        let tipo = if eh_num { "number" } else { "text" };
        view! {
            <input
                class="input input--num"
                type=tipo
                prop:disabled=!eh_gestor
                prop:value=valor
                on:input=move |ev| {
                    let v = event_target_value(&ev);
                    edits.update(|m| {
                        m.insert(p2.clone(), v);
                    });
                }
            />
        }
        .into_any()
    };
    view! {
        <label class="config-campo">
            <span class="config-campo__rotulo">{rotulo}</span>
            {entrada}
        </label>
    }
}

#[component]
fn AuditoriaConfig() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let trilha = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => auditoria_config(t).await.unwrap_or_default(),
                None => Vec::new(),
            }
        },
    );
    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Auditoria de mudanças"</h2>
                <p class="texto-suave">"Quem alterou, quando e o valor anterior (§7.5)."</p>
            </header>
            <Suspense fallback=|| view! { <p class="texto-suave">"Carregando…"</p> }>
                {move || {
                    let itens: Vec<EntradaAuditoriaConfig> = trilha.get().unwrap_or_default();
                    if itens.is_empty() {
                        view! { <p class="estado-vazio">"Nenhuma mudança registrada."</p> }.into_any()
                    } else {
                        view! {
                            <ul class="solic-lista">
                                {itens
                                    .into_iter()
                                    .map(|e| {
                                        let de = e.valor_anterior.unwrap_or_else(|| "—".to_owned());
                                        let para = e.valor_novo.unwrap_or_else(|| "—".to_owned());
                                        view! {
                                            <li class="solic-item">
                                                <div class="solic-item__dados">
                                                    <span class="solic-item__qtd">{e.chave}</span>
                                                    <span class="texto-suave">
                                                        {format!("{de} → {para}")}
                                                    </span>
                                                </div>
                                                <span class="texto-suave">{e.em}</span>
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

/// Achata um JSON em pares `caminho → valor` (folhas escalares).
fn achatar(prefixo: &str, v: &Value, out: &mut Vec<(String, Value)>) {
    if let Value::Object(mapa) = v {
        for (k, val) in mapa {
            let caminho = if prefixo.is_empty() {
                k.clone()
            } else {
                format!("{prefixo}.{k}")
            };
            achatar(&caminho, val, out);
        }
    } else if !v.is_array() {
        out.push((prefixo.to_owned(), v.clone()));
    }
}

/// Valor inicial do input (texto): bool → `true/false`; número → dígitos; string → conteúdo.
fn valor_para_input(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Reconstrói o JSON a partir do original, aplicando as edições com o tipo de cada folha.
fn reconstruir(
    original: &Value,
    leaves: &[(String, Value)],
    edits: &BTreeMap<String, String>,
) -> Value {
    let mut out = original.clone();
    for (path, orig) in leaves {
        let Some(s) = edits.get(path) else { continue };
        let novo = match orig {
            Value::Bool(_) => Value::Bool(s == "true"),
            Value::Number(n) if n.is_f64() => {
                serde_json::Number::from_f64(s.parse().unwrap_or(0.0))
                    .map_or_else(|| orig.clone(), Value::Number)
            }
            Value::Number(_) => s.parse::<i64>().map_or_else(|_| orig.clone(), Value::from),
            _ => Value::String(s.clone()),
        };
        set_path(&mut out, path, novo);
    }
    out
}

/// Define o valor de uma folha pelo caminho `a.b.c` (os nós existem — vêm do próprio original).
fn set_path(v: &mut Value, path: &str, novo: Value) {
    let partes: Vec<&str> = path.split('.').collect();
    let mut atual = v;
    for (i, parte) in partes.iter().enumerate() {
        let Value::Object(mapa) = atual else { return };
        if i + 1 == partes.len() {
            mapa.insert((*parte).to_owned(), novo);
            return;
        }
        let Some(prox) = mapa.get_mut(*parte) else {
            return;
        };
        atual = prox;
    }
}
