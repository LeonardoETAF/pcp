//! Acesso ao `localStorage` do navegador para preferências leves do cliente (ex.: "Lembrar-me"
//! guardando o e-mail — nunca a senha, §7). Só faz algo no build WASM; no SSR é no-op (não há
//! `window`). Mesmo padrão de `download.rs`.

/// Chave do e-mail lembrado na tela de login ("Lembrar-me").
pub const EMAIL_LEMBRADO: &str = "pcp_email_lembrado";

/// Chave do refresh token persistido — restaura a sessão após reload (§7: refresh token opaco,
/// não a senha; endurecimento futuro = cookie httpOnly).
pub const REFRESH: &str = "pcp_refresh";

#[cfg(target_arch = "wasm32")]
fn storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok().flatten()
}

/// Lê um valor do `localStorage` (`None` se ausente ou fora do navegador).
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn ler(chave: &str) -> Option<String> {
    storage()?.get_item(chave).ok().flatten()
}

/// Grava um valor no `localStorage` (silencioso em falha — recurso opcional).
#[cfg(target_arch = "wasm32")]
pub fn gravar(chave: &str, valor: &str) {
    if let Some(s) = storage() {
        let _ = s.set_item(chave, valor);
    }
}

/// Remove uma chave do `localStorage`.
#[cfg(target_arch = "wasm32")]
pub fn remover(chave: &str) {
    if let Some(s) = storage() {
        let _ = s.remove_item(chave);
    }
}

/// No-op no servidor (SSR): o `localStorage` só existe no cliente.
#[cfg(not(target_arch = "wasm32"))]
#[must_use]
pub fn ler(_chave: &str) -> Option<String> {
    None
}

/// No-op no servidor (SSR).
#[cfg(not(target_arch = "wasm32"))]
pub fn gravar(_chave: &str, _valor: &str) {}

/// No-op no servidor (SSR).
#[cfg(not(target_arch = "wasm32"))]
pub fn remover(_chave: &str) {}
