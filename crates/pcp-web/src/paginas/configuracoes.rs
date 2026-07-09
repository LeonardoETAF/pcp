//! Configurações (doc 03 §8): edição de TODAS as constantes de negócio (doc 02 §11) com
//! validação no servidor, persistência e recarga a quente, mais a trilha de auditoria (§7.5).
//! Frontend burro (§3): trata a config como JSON opaco — não conhece a regra; só edita valores.

use std::collections::BTreeMap;

use leptos::prelude::*;
use serde_json::Value;

use crate::api::{
    atualizar_usuario, auditoria_config, criar_usuario, listar_sazonalidade, listar_usuarios,
    obter_config, obter_preferencias, override_sazonalidade, perfil, salvar_config,
    salvar_preferencias, EntradaAuditoriaConfig, UsuarioConta,
};
use crate::componentes::EstadoVazio;
use crate::componentes::Seletor;
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
            <Preferencias />
            <Sazonalidade />
            <Usuarios />
        </section>
    }
}

/// Preferências de exibição do próprio usuário (doc 03 §8): página inicial e tamanho de página.
#[component]
fn Preferencias() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let msg = RwSignal::new(None::<String>);
    let pagina = RwSignal::new("dashboard".to_owned());
    let tamanho = RwSignal::new(50_i32);

    let prefs = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => obter_preferencias(t).await.ok(),
                None => None,
            }
        },
    );
    Effect::new(move |_| {
        if let Some(Some(p)) = prefs.get() {
            pagina.set(p.pagina_inicial);
            tamanho.set(p.tamanho_pagina);
        }
    });

    let salvar = move |_| {
        let Some(token) = sessao.0.get_untracked() else {
            return;
        };
        let (pi, tp) = (pagina.get_untracked(), tamanho.get_untracked());
        leptos::task::spawn_local(async move {
            match salvar_preferencias(token, pi, tp).await {
                Ok(_) => msg.set(Some("Preferências salvas.".to_owned())),
                Err(e) => msg.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Minhas preferências"</h2>
                <p class="texto-suave">"Página inicial e tamanho de página."</p>
            </header>
            <div class="solic-form">
                <div class="campo-select">
                    <span class="campo-select__rotulo">"Página inicial"</span>
                    <Seletor
                        rotulo="Página inicial"
                        opcoes=vec![
                            ("dashboard", "Dashboard"),
                            ("estoque", "Estoque"),
                            ("alertas", "Alertas"),
                            ("abc", "Classificação ABC"),
                        ]
                        valor=Signal::derive(move || pagina.get())
                        ao_escolher=move |v| pagina.set(v)
                    />
                </div>
                <div class="campo-select">
                    <span class="campo-select__rotulo">"Tamanho de página"</span>
                    <Seletor
                        rotulo="Tamanho de página"
                        opcoes=vec![("50", "50"), ("100", "100"), ("500", "500"), ("1000", "1000")]
                        valor=Signal::derive(move || tamanho.get().to_string())
                        ao_escolher=move |v: String| {
                            if let Ok(n) = v.parse::<i32>() {
                                tamanho.set(n);
                            }
                        }
                    />
                </div>
                <button type="button" class="btn btn--primario" on:click=salvar>
                    "Salvar"
                </button>
            </div>
            {move || msg.get().map(|m| view! { <p class="texto-suave">{m}</p> })}
        </section>
    }
}

fn nome_mes(mes: i16) -> &'static str {
    match mes {
        1 => "Jan",
        2 => "Fev",
        3 => "Mar",
        4 => "Abr",
        5 => "Mai",
        6 => "Jun",
        7 => "Jul",
        8 => "Ago",
        9 => "Set",
        10 => "Out",
        11 => "Nov",
        12 => "Dez",
        _ => "—",
    }
}

/// Fatores sazonais (doc 03 §8): vigentes + override manual com justificativa (gestor).
#[component]
fn Sazonalidade() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let recarregar = RwSignal::new(0_u32);
    let justificativa = RwSignal::new(String::new());
    let msg = RwSignal::new(None::<String>);
    let edits = RwSignal::new(BTreeMap::<i16, String>::new());

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

    let fatores = Resource::new(
        move || (sessao.0.get(), recarregar.get()),
        |(t, _)| async move {
            match t {
                Some(t) => listar_sazonalidade(t).await.unwrap_or_default(),
                None => Vec::new(),
            }
        },
    );
    // Inicializa os campos a partir dos fatores carregados (12 meses; default 1.00).
    Effect::new(move |_| {
        let mapa: BTreeMap<i16, f64> = fatores
            .get()
            .unwrap_or_default()
            .into_iter()
            .map(|f| (f.mes, f.fator))
            .collect();
        edits.set(
            (1..=12_i16)
                .map(|mes| {
                    (
                        mes,
                        format!("{:.2}", mapa.get(&mes).copied().unwrap_or(1.0)),
                    )
                })
                .collect(),
        );
    });

    let salvar = move |mes: i16| {
        let Some(token) = sessao.0.get_untracked() else {
            return;
        };
        let Some(valor) = edits
            .get_untracked()
            .get(&mes)
            .and_then(|s| s.parse::<f64>().ok())
        else {
            msg.set(Some("Informe um fator numérico.".to_owned()));
            return;
        };
        let just = justificativa.get_untracked();
        leptos::task::spawn_local(async move {
            match override_sazonalidade(token, mes, valor, just).await {
                Ok(_) => {
                    msg.set(Some(format!("Fator de {} salvo.", nome_mes(mes))));
                    recarregar.update(|n| *n += 1);
                }
                Err(e) => msg.set(Some(e.to_string())),
            }
        });
    };

    view! {
        <section class="cartao">
            <header class="cartao__cab">
                <h2 class="cartao__titulo">"Sazonalidade"</h2>
                <p class="texto-suave">"Fatores vigentes por mês; override manual (gestor)."</p>
            </header>
            {move || msg.get().map(|m| view! { <p class="texto-suave">{m}</p> })}
            <Show when=eh_gestor>
                <label class="campo-select">
                    <span class="campo-select__rotulo">"Justificativa do override"</span>
                    <input
                        class="input"
                        placeholder="Opcional"
                        prop:value=move || justificativa.get()
                        on:input=move |ev| justificativa.set(event_target_value(&ev))
                    />
                </label>
            </Show>
            {move || {
                let gestor = eh_gestor();
                view! {
                    <div class="cobertura-classes">
                        {(1..=12_i16)
                            .map(|mes| {
                                view! {
                                    <div class="cob-classe">
                                        <span class="config-campo__rotulo">{nome_mes(mes)}</span>
                                        <input
                                            class="input input--num"
                                            type="number"
                                            step="0.01"
                                            prop:disabled=!gestor
                                            prop:value=move || {
                                                edits.get().get(&mes).cloned().unwrap_or_default()
                                            }
                                            on:input=move |ev| {
                                                let v = event_target_value(&ev);
                                                edits.update(|m| {
                                                    m.insert(mes, v);
                                                });
                                            }
                                        />
                                        {gestor
                                            .then(|| {
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="btn btn--secundario btn--sm"
                                                        on:click=move |_| salvar(mes)
                                                    >
                                                        "Salvar"
                                                    </button>
                                                }
                                            })}
                                    </div>
                                }
                            })
                            .collect_view()}
                    </div>
                }
            }}
        </section>
    }
}

/// Gestão de usuários e papéis (doc 03 §8) — somente admin.
#[component]
#[allow(clippy::too_many_lines)] // tabela + formulário de criação
fn Usuarios() -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let recarregar = RwSignal::new(0_u32);
    let msg = RwSignal::new(None::<String>);
    let (n_email, n_senha, n_papel, n_nome) = (
        RwSignal::new(String::new()),
        RwSignal::new(String::new()),
        RwSignal::new("analista".to_owned()),
        RwSignal::new(String::new()),
    );

    let papel = Resource::new(
        move || sessao.0.get(),
        |t| async move {
            match t {
                Some(t) => perfil(t).await.unwrap_or_default(),
                None => String::new(),
            }
        },
    );
    let eh_admin = move || matches!(papel.get().as_deref(), Some("admin"));

    let lista = Resource::new(
        move || (sessao.0.get(), recarregar.get()),
        |(t, _)| async move {
            match t {
                Some(t) => listar_usuarios(t).await.unwrap_or_default(),
                None => Vec::new(),
            }
        },
    );

    let criar = move |_| {
        let Some(token) = sessao.0.get_untracked() else {
            return;
        };
        let (e, s, p, n) = (
            n_email.get_untracked(),
            n_senha.get_untracked(),
            n_papel.get_untracked(),
            n_nome.get_untracked(),
        );
        leptos::task::spawn_local(async move {
            match criar_usuario(token, e, s, p, n).await {
                Ok(()) => {
                    n_email.set(String::new());
                    n_senha.set(String::new());
                    n_nome.set(String::new());
                    msg.set(Some("Usuário criado.".to_owned()));
                    recarregar.update(|x| *x += 1);
                }
                Err(err) => msg.set(Some(err.to_string())),
            }
        });
    };

    view! {
        <Show when=eh_admin>
            <section class="cartao">
                <header class="cartao__cab">
                    <h2 class="cartao__titulo">"Usuários e papéis"</h2>
                    <p class="texto-suave">"Somente admin (CLAUDE.md §7.3)."</p>
                </header>
                {move || msg.get().map(|m| view! { <p class="texto-suave">{m}</p> })}
                <div class="solic-form">
                    <input
                        class="input"
                        placeholder="E-mail"
                        prop:value=move || n_email.get()
                        on:input=move |ev| n_email.set(event_target_value(&ev))
                    />
                    <input
                        class="input"
                        type="password"
                        placeholder="Senha"
                        prop:value=move || n_senha.get()
                        on:input=move |ev| n_senha.set(event_target_value(&ev))
                    />
                    <input
                        class="input"
                        placeholder="Nome (opcional)"
                        prop:value=move || n_nome.get()
                        on:input=move |ev| n_nome.set(event_target_value(&ev))
                    />
                    <Seletor
                        rotulo="Papel do novo usuário"
                        opcoes=vec![
                            ("analista", "Analista"),
                            ("gestor", "Gestor"),
                            ("admin", "Admin"),
                        ]
                        valor=Signal::derive(move || n_papel.get())
                        ao_escolher=move |v| n_papel.set(v)
                    />
                    <button type="button" class="btn btn--primario" on:click=criar>
                        "Criar"
                    </button>
                </div>
                <Suspense fallback=|| view! { <p class="texto-suave">"Carregando…"</p> }>
                    {move || {
                        let itens = lista.get().unwrap_or_default();
                        view! {
                            <ul class="solic-lista">
                                {itens
                                    .into_iter()
                                    .map(|u| view! { <LinhaUsuario u recarregar /> })
                                    .collect_view()}
                            </ul>
                        }
                    }}
                </Suspense>
            </section>
        </Show>
    }
}

#[component]
fn LinhaUsuario(u: UsuarioConta, recarregar: RwSignal<u32>) -> impl IntoView {
    let sessao = expect_context::<Sessao>();
    let papel = RwSignal::new(u.papel.clone());
    let ativo = RwSignal::new(u.ativo);
    let id = StoredValue::new(u.id.clone());
    let salvar = move |_| {
        let Some(token) = sessao.0.get_untracked() else {
            return;
        };
        let (p, a) = (papel.get_untracked(), ativo.get_untracked());
        leptos::task::spawn_local(async move {
            let _ = atualizar_usuario(token, id.get_value(), p, a).await;
            recarregar.update(|n| *n += 1);
        });
    };
    view! {
        <li class="solic-item">
            <div class="solic-item__dados">
                <span class="solic-item__qtd">{u.email.clone()}</span>
                <span class="texto-suave">{u.nome.clone().unwrap_or_default()}</span>
            </div>
            <div class="solic-item__acoes">
                <Seletor
                    rotulo="Papel"
                    opcoes=vec![
                        ("analista", "Analista"),
                        ("gestor", "Gestor"),
                        ("admin", "Admin"),
                    ]
                    valor=Signal::derive(move || papel.get())
                    ao_escolher=move |v| papel.set(v)
                />
                <label class="switch">
                    <input
                        type="checkbox"
                        prop:checked=move || ativo.get()
                        on:change=move |ev| ativo.set(event_target_checked(&ev))
                    />
                    <span>"Ativo"</span>
                </label>
                <button type="button" class="btn btn--secundario btn--sm" on:click=salvar>
                    "Salvar"
                </button>
            </div>
        </li>
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
                    let n = campos.len();
                    view! {
                        <section class="cartao">
                            <header class="config-secao__cab">
                                <span class="config-secao__dot"></span>
                                <h2 class="config-secao__nome">{secao}</h2>
                                <span class="config-secao__contagem">
                                    {format!("{n} parâmetros")}
                                </span>
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
    // Número → input numérico; booleano e texto → campo de texto (booleano edita "true"/"false",
    // como no mockup; `reconstruir` reconverte ao tipo certo da folha).
    let tipo = if orig.is_number() { "number" } else { "text" };
    view! {
        <label class="config-campo">
            <span class="config-campo__rotulo">{rotulo}</span>
            <input
                class="input"
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
                        view! {
                            <EstadoVazio
                                arte="empty-movements.svg"
                                titulo="Nenhuma mudança registrada"
                                descricao="Toda edição de configuração é auditada e listada aqui."
                            />
                        }
                            .into_any()
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
