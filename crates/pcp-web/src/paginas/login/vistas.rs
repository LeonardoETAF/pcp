//! Vistas do painel direito: entrar, recuperar senha e falar com o administrador.
//! Só o login tem backend (server function `login`). Recuperar/Contato são UI: não há endpoint
//! de e-mail/solicitação ainda, então seus formulários não enviam (sem ação simulada — §13).

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;

use super::Vista;
use crate::api::{obter_preferencias, Login};
use crate::contexto::Sessao;

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

    view! {
        <div class="vista">
            <h1 class="vista__titulo vista__titulo--centro">"Bem-vindo"</h1>
            <p class="vista__sub vista__sub--centro">"Entre com suas credenciais."</p>
            <ActionForm action=login attr:class="form-auth">
                <div class="campo-auth">
                    <label class="campo-auth__rotulo">"E-mail"</label>
                    <div class="input-wrap">
                        <span class="input-wrap__icone">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                                <rect x="3" y="5" width="18" height="14" rx="2" />
                                <path d="m3 7 9 6 9-6" />
                            </svg>
                        </span>
                        <input
                            class="input-auth"
                            type="email"
                            name="email"
                            placeholder="voce@empresa.com.br"
                            autocomplete="username"
                            prop:value=move || email.get()
                            on:input=move |ev| email.set(event_target_value(&ev))
                        />
                    </div>
                </div>
                <div class="campo-auth">
                    <label class="campo-auth__rotulo">"Senha"</label>
                    <div class="input-wrap">
                        <span class="input-wrap__icone">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                                <rect x="4" y="10" width="16" height="10" rx="2" />
                                <path d="M8 10V7a4 4 0 0 1 8 0v3" />
                            </svg>
                        </span>
                        <input
                            class="input-auth input-auth--olho"
                            type=move || if mostrar_senha.get() { "text" } else { "password" }
                            name="senha"
                            placeholder="Sua senha"
                            autocomplete="current-password"
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
                </div>
                // "Lembrar" à esquerda e "Esqueci a senha" à direita, abaixo do campo de senha.
                <div class="lembrar-linha">
                    <label class="lembrar">
                        <input
                            type="checkbox"
                            prop:checked=move || lembrar.get()
                            on:change=move |ev| lembrar.set(event_target_checked(&ev))
                        />
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
                <button class="btn-auth" type="submit" prop:disabled=move || login.pending().get()>
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
    view! {
        <div class="vista">
            <BotaoVoltar vista />
            <h1 class="vista__titulo">"Recuperar senha"</h1>
            <p class="vista__sub">"Informe seu e-mail e enviaremos um link para redefinir."</p>
            <form class="form-auth" on:submit=move |ev| ev.prevent_default()>
                <div class="campo-auth">
                    <label class="campo-auth__rotulo">"E-mail"</label>
                    <div class="input-wrap">
                        <span class="input-wrap__icone">
                            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                                <rect x="3" y="5" width="18" height="14" rx="2" />
                                <path d="m3 7 9 6 9-6" />
                            </svg>
                        </span>
                        <input
                            class="input-auth"
                            type="email"
                            placeholder="voce@empresa.com.br"
                            autocomplete="username"
                        />
                    </div>
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
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                            <rect x="3" y="5" width="18" height="14" rx="2" />
                            <path d="m3 7 9 6 9-6" />
                        </svg>
                    </span>
                    <div>
                        <p class="card-info__rotulo">"E-mail"</p>
                        <p class="card-info__valor">"ti@supercopo.com.br"</p>
                    </div>
                </div>
                <div class="card-info">
                    <span class="card-info__icone">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M5 4h4l2 5-2.5 1.5a11 11 0 0 0 5 5L16 13l5 2v4a2 2 0 0 1-2 2A16 16 0 0 1 3 6a2 2 0 0 1 2-2Z" />
                        </svg>
                    </span>
                    <div>
                        <p class="card-info__rotulo">"Telefone"</p>
                        <p class="card-info__valor">"(11) 4002-8922 · ramal 120"</p>
                    </div>
                </div>
                <div class="card-info">
                    <span class="card-info__icone">
                        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                            <circle cx="12" cy="12" r="9" />
                            <path d="M12 7v5l3 2" />
                        </svg>
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
            <form class="form-auth" on:submit=move |ev| ev.prevent_default()>
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
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
                <path d="m15 18-6-6 6-6" />
            </svg>
            "Voltar ao login"
        </button>
    }
}
