//! Vistas do painel direito: entrar, recuperar senha e falar com o administrador.
//! Só o login tem backend (server function `login`). Recuperar/Contato são UI: não há endpoint
//! de e-mail/solicitação ainda, então seus formulários não enviam (sem ação simulada — §13).

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;

use super::{Icone, Vista};
use crate::api::{obter_preferencias, Login};
use crate::contexto::Sessao;

// Mensagens exibidas PELO SISTEMA (nada de balão do navegador), uma por situação de cada campo.
const MSG_EMAIL_VAZIO: &str = "Informe seu e-mail.";
const MSG_EMAIL_INVALIDO: &str =
    "Informe um e-mail válido, com @ e domínio (ex.: nome@empresa.com.br).";
const MSG_SENHA_VAZIA: &str = "Informe sua senha.";

/// Valida o e-mail NO SISTEMA (sem depender do navegador): exige parte local, `@`, domínio com
/// ponto e TLD alfabético de 2+ letras, sem espaços. Não exige literalmente `.com` — isso barraria
/// domínios válidos como `supercopo.local` (o admin do projeto) ou `.com.br`.
fn email_valido(e: &str) -> bool {
    let e = e.trim();
    if e.is_empty() || e.chars().any(char::is_whitespace) {
        return false;
    }
    let Some((local, dominio)) = e.split_once('@') else {
        return false;
    };
    if local.is_empty() || dominio.contains('@') {
        return false;
    }
    let Some((host, tld)) = dominio.rsplit_once('.') else {
        return false;
    };
    !host.is_empty() && tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_alphabetic())
}

/// Vista de entrada (login real via server function).
#[component]
#[allow(clippy::too_many_lines)] // view declarativa do formulário de login
pub fn VistaLogin(vista: RwSignal<Vista>) -> impl IntoView {
    let login = ServerAction::<Login>::new();
    let sessao = expect_context::<Sessao>();
    let navegar = StoredValue::new(use_navigate());
    let mostrar_senha = RwSignal::new(false);
    // "Lembrar-me": guarda só o e-mail no navegador (a senha fica com o gerenciador do
    // navegador via autocomplete — nunca em localStorage, §7).
    let email = RwSignal::new(String::new());
    let lembrar = RwSignal::new(false);

    // Ao montar (cliente), preenche o e-mail lembrado e marca o checkbox.
    Effect::new(move |_| {
        if let Some(salvo) = crate::armazenamento::ler(crate::armazenamento::EMAIL_LEMBRADO) {
            email.set(salvo);
            lembrar.set(true);
        }
    });

    Effect::new(move |_| {
        if let Some(Ok(cred)) = login.value().get() {
            // Persiste/limpa o e-mail conforme "Lembrar-me".
            if lembrar.get_untracked() {
                crate::armazenamento::gravar(
                    crate::armazenamento::EMAIL_LEMBRADO,
                    &email.get_untracked(),
                );
            } else {
                crate::armazenamento::remover(crate::armazenamento::EMAIL_LEMBRADO);
            }
            // Persiste o refresh token para restaurar a sessão após reload (§7).
            crate::armazenamento::gravar(crate::armazenamento::REFRESH, &cred.refresh_token);
            sessao.0.set(Some(cred.access_token.clone()));
            // Redireciona para a página inicial preferida do usuário (doc 03 §8).
            let token = cred.access_token;
            leptos::task::spawn_local(async move {
                let destino = obter_preferencias(token)
                    .await
                    .map_or_else(|_| "dashboard".to_owned(), |p| p.pagina_inicial);
                navegar.with_value(|n| n(&format!("/{destino}"), NavigateOptions::default()));
            });
        }
    });
    let tem_erro = move || matches!(login.value().get(), Some(Err(_)));
    // Validação do sistema (não do navegador). As mensagens só aparecem DEPOIS de tentar entrar —
    // nunca enquanto o usuário digita; ao voltar a digitar, os avisos somem. Cada campo tem a sua.
    let senha = RwSignal::new(String::new());
    let tentou = RwSignal::new(false);
    let erro_email = move || {
        if !tentou.get() {
            return None;
        }
        let e = email.get();
        if e.trim().is_empty() {
            Some(MSG_EMAIL_VAZIO)
        } else if email_valido(&e) {
            None
        } else {
            Some(MSG_EMAIL_INVALIDO)
        }
    };
    // A senha não é aparada: espaços podem fazer parte dela.
    let erro_senha = move || (tentou.get() && senha.get().is_empty()).then_some(MSG_SENHA_VAZIA);
    let email_ruim = move || erro_email().is_some();

    view! {
        <div class="vista">
            <h1 class="vista__titulo vista__titulo--centro">"Bem-vindo"</h1>
            <p class="vista__sub vista__sub--centro">"Entre com suas credenciais."</p>
            // `novalidate`: desliga os balões nativos — as mensagens são renderizadas pelo sistema.
            <ActionForm action=login attr:class="form-auth" attr:novalidate=true>
                <div class="campo-auth">
                    <label class="campo-auth__rotulo">"E-mail"</label>
                    <div class="input-wrap">
                        <span class="input-wrap__icone">
                            <Icone arquivo="email.svg" />
                        </span>
                        <input
                            class="input-auth"
                            class:input-auth--invalido=email_ruim
                            type="email"
                            name="email"
                            placeholder="voce@empresa.com.br"
                            autocomplete="username"
                            aria-invalid=move || email_ruim().then_some("true")
                            prop:value=move || email.get()
                            on:input=move |ev| {
                                email.set(event_target_value(&ev));
                                tentou.set(false);
                            }
                        />
                    </div>
                    {move || {
                        erro_email()
                            .map(|m| view! { <p class="campo-auth__erro" role="alert">{m}</p> })
                    }}
                </div>
                <div class="campo-auth">
                    <label class="campo-auth__rotulo">"Senha"</label>
                    <div class="input-wrap">
                        <span class="input-wrap__icone">
                            <Icone arquivo="seguranca.svg" />
                        </span>
                        <input
                            class="input-auth input-auth--olho"
                            class:input-auth--invalido=move || erro_senha().is_some()
                            type=move || if mostrar_senha.get() { "text" } else { "password" }
                            name="senha"
                            placeholder="Sua senha"
                            autocomplete="current-password"
                            aria-invalid=move || erro_senha().map(|_| "true")
                            prop:value=move || senha.get()
                            on:input=move |ev| {
                                senha.set(event_target_value(&ev));
                                tentou.set(false);
                            }
                        />
                        <button
                            class="input-wrap__olho"
                            type="button"
                            aria-label="Mostrar ou ocultar a senha"
                            on:click=move |_| mostrar_senha.update(|m| *m = !*m)
                        >
                            <span
                                class="icone-mask"
                                style=move || {
                                    let arq = if mostrar_senha.get() {
                                        "ocultar.svg"
                                    } else {
                                        "visualizar.svg"
                                    };
                                    format!(
                                        "-webkit-mask-image:url(/icons/{arq});mask-image:url(/icons/{arq})",
                                    )
                                }
                            ></span>
                        </button>
                    </div>
                    {move || {
                        erro_senha()
                            .map(|m| view! { <p class="campo-auth__erro" role="alert">{m}</p> })
                    }}
                </div>
                // "Lembrar" à esquerda e "Esqueci a senha" à direita, abaixo do campo de senha.
                <div class="lembrar-linha">
                    // O <input> real fica sob a caixa desenhada: preserva teclado, foco e leitor
                    // de tela; quem aparece é a caixa, que o navegador não pinta.
                    <label class="lembrar">
                        <input
                            class="lembrar__entrada"
                            type="checkbox"
                            prop:checked=move || lembrar.get()
                            on:change=move |ev| lembrar.set(event_target_checked(&ev))
                        />
                        <span class="lembrar__caixa" aria-hidden="true">
                            <Icone arquivo="confirmar.svg" />
                        </span>
                        <span>"Lembrar"</span>
                    </label>
                    <button
                        class="link-suave"
                        type="button"
                        on:click=move |_| vista.set(Vista::Recuperar)
                    >
                        "Esqueci a senha"
                    </button>
                </div>
                {move || {
                    tem_erro()
                        .then(|| view! { <p class="form-auth__erro" role="alert">"Credenciais inválidas."</p> })
                }}
                // O bloqueio é no CLIQUE (o submit é a ação padrão dele). Não dá para interceptar
                // pelo `on:submit` do ActionForm: o handler dele é registrado antes e já despacha.
                <button
                    class="btn-auth"
                    type="submit"
                    disabled=move || login.pending().get()
                    on:click=move |ev| {
                        tentou.set(true);
                        let email_ok = email_valido(&email.get_untracked());
                        let senha_ok = !senha.get_untracked().is_empty();
                        if !email_ok || !senha_ok {
                            ev.prevent_default();
                        }
                    }
                >
                    {move || if login.pending().get() { "Entrando…" } else { "Entrar" }}
                </button>
            </ActionForm>
            <p class="vista__rodape">
                "Problemas para acessar? "
                <button class="link-forte" type="button" on:click=move |_| vista.set(Vista::Contato)>
                    "Fale com o administrador"
                </button>
            </p>
        </div>
    }
}

/// Vista de recuperação de senha (UI; backend de e-mail ainda não implementado).
#[component]
pub fn VistaRecuperar(vista: RwSignal<Vista>) -> impl IntoView {
    let email = RwSignal::new(String::new());
    // Mesma regra do login: o aviso só surge ao tentar enviar, não enquanto digita.
    let tentou = RwSignal::new(false);
    let erro_email = move || {
        if !tentou.get() {
            return None;
        }
        let e = email.get();
        if e.trim().is_empty() {
            Some(MSG_EMAIL_VAZIO)
        } else if email_valido(&e) {
            None
        } else {
            Some(MSG_EMAIL_INVALIDO)
        }
    };
    let email_ruim = move || erro_email().is_some();
    view! {
        <div class="vista">
            <BotaoVoltar vista />
            <h1 class="vista__titulo">"Recuperar senha"</h1>
            <p class="vista__sub">"Informe seu e-mail e enviaremos um link para redefinir."</p>
            // `novalidate`: sem balões nativos — a mensagem é do sistema.
            <form
                class="form-auth"
                novalidate
                on:submit=move |ev| {
                    ev.prevent_default();
                    tentou.set(true);
                }
            >
                <div class="campo-auth">
                    <label class="campo-auth__rotulo">"E-mail"</label>
                    <div class="input-wrap">
                        <span class="input-wrap__icone">
                            <Icone arquivo="email.svg" />
                        </span>
                        <input
                            class="input-auth"
                            class:input-auth--invalido=email_ruim
                            type="email"
                            placeholder="voce@empresa.com.br"
                            autocomplete="username"
                            aria-invalid=move || email_ruim().then_some("true")
                            prop:value=move || email.get()
                            on:input=move |ev| {
                                email.set(event_target_value(&ev));
                                tentou.set(false);
                            }
                        />
                    </div>
                    {move || {
                        erro_email()
                            .map(|m| view! { <p class="campo-auth__erro" role="alert">{m}</p> })
                    }}
                </div>
                <button class="btn-auth" type="submit">
                    "Enviar link de redefinição"
                </button>
            </form>
        </div>
    }
}

/// Vista "falar com o administrador": canais de contato + formulário de solicitação (UI).
#[component]
pub fn VistaContato(vista: RwSignal<Vista>) -> impl IntoView {
    view! {
        <div class="vista">
            <BotaoVoltar vista />
            <h1 class="vista__titulo">"Falar com o administrador"</h1>
            <p class="vista__sub">
                "Precisa de acesso, redefinição de senha ou está com algum problema? Use um dos canais abaixo ou envie uma solicitação."
            </p>
            <div class="cards-info">
                <div class="card-info">
                    <span class="card-info__icone">
                        <Icone arquivo="email.svg" />
                    </span>
                    <div>
                        <p class="card-info__rotulo">"E-mail"</p>
                        <p class="card-info__valor">"ti@supercopo.com.br"</p>
                    </div>
                </div>
                <div class="card-info">
                    <span class="card-info__icone">
                        <Icone arquivo="telefone.svg" />
                    </span>
                    <div>
                        <p class="card-info__rotulo">"Telefone"</p>
                        <p class="card-info__valor">"(11) 4002-8922 · ramal 120"</p>
                    </div>
                </div>
                <div class="card-info">
                    <span class="card-info__icone">
                        <Icone arquivo="relogio.svg" />
                    </span>
                    <div>
                        <p class="card-info__rotulo">"Atendimento"</p>
                        <p class="card-info__valor">"Seg a Sex, 8h às 18h"</p>
                    </div>
                </div>
            </div>
            <div class="divisor">
                <span>"ou envie uma solicitação"</span>
            </div>
            <form class="form-auth" novalidate on:submit=move |ev| ev.prevent_default()>
                <input class="input-auth input-auth--solo" placeholder="Seu nome" />
                <input class="input-auth input-auth--solo" type="email" placeholder="Seu e-mail" />
                <textarea class="textarea-auth" placeholder="Descreva o que você precisa…"></textarea>
                <button class="btn-auth" type="submit">
                    "Enviar solicitação"
                </button>
            </form>
        </div>
    }
}

/// Botão "‹ Voltar ao login" reutilizado pelas vistas secundárias.
#[component]
fn BotaoVoltar(vista: RwSignal<Vista>) -> impl IntoView {
    view! {
        <button class="voltar" type="button" on:click=move |_| vista.set(Vista::Login)>
            <Icone arquivo="seta-esquerda.svg" />
            "Voltar ao login"
        </button>
    }
}
