//! Tela de login: dispara a server function `login` e, em sucesso, guarda o token na sessão e
//! navega para a home. Frontend burro (CLAUDE.md §3).

use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;

use crate::api::Login;
use crate::contexto::Sessao;

#[component]
pub fn PaginaLogin() -> impl IntoView {
    let login = ServerAction::<Login>::new();
    let sessao = expect_context::<Sessao>();
    let navegar = use_navigate();

    Effect::new(move |_| {
        if let Some(Ok(token)) = login.value().get() {
            sessao.0.set(Some(token));
            navegar("/", NavigateOptions::default());
        }
    });

    let mensagem_erro = move || match login.value().get() {
        Some(Err(e)) => Some(e.to_string()),
        _ => None,
    };

    view! {
        <main class="login">
            <section class="login__cartao card">
                <img class="login__logo" src="/images/logo.svg" alt="SuperCopo PCP" />
                <h1 class="login__titulo">"Entrar no PCP"</h1>
                <p class="login__sub">"Planejamento e Controle de Produção"</p>
                <ActionForm action=login attr:class="login__form">
                    <label class="campo">
                        <span class="campo__rotulo">"E-mail"</span>
                        <input class="input" type="email" name="email" autocomplete="username" />
                    </label>
                    <label class="campo">
                        <span class="campo__rotulo">"Senha"</span>
                        <input
                            class="input"
                            type="password"
                            name="senha"
                            autocomplete="current-password"
                        />
                    </label>
                    {move || {
                        mensagem_erro()
                            .map(|e| view! { <p class="login__erro" role="alert">{e}</p> })
                    }}
                    <button
                        class="btn btn--primario"
                        type="submit"
                        prop:disabled=move || login.pending().get()
                    >
                        {move || if login.pending().get() { "Entrando…" } else { "Entrar" }}
                    </button>
                </ActionForm>
            </section>
        </main>
    }
}
