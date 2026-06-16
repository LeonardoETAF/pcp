//! Ponto de entrada da hidratação WASM. EXCEÇÃO §3/§5 (autorizada): o `allow(unsafe_code)` do
//! projeto vive SÓ aqui, por causa da cola gerada pela macro `#[wasm_bindgen]`. Nenhum `unsafe`
//! escrito à mão; sob a feature `ssr` este módulo fica vazio.
#![allow(unsafe_code)]

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(crate::app::App);
}
