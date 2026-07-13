//! Download de um arquivo gerado no cliente (exportação CSV/JSON — doc 03 §3 / §12). Cria um
//! Blob a partir do conteúdo e dispara o download via `<a download>`. Só faz algo no build WASM;
//! no SSR é no-op (a função roda em handler de clique, sempre no cliente).

/// Dispara o download de `conteudo` com o nome de arquivo `nome`.
#[cfg(target_arch = "wasm32")]
pub fn baixar(nome: &str, conteudo: &str) {
    if let Err(e) = tentar_baixar(nome, conteudo) {
        leptos::logging::error!("falha ao baixar arquivo: {e:?}");
    }
}

#[cfg(target_arch = "wasm32")]
fn tentar_baixar(nome: &str, conteudo: &str) -> Result<(), wasm_bindgen::JsValue> {
    use wasm_bindgen::{JsCast, JsValue};

    let partes = js_sys::Array::of1(&JsValue::from_str(conteudo));
    let blob = web_sys::Blob::new_with_str_sequence(&partes)?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)?;

    let documento = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| JsValue::from_str("sem document"))?;
    let ancora: web_sys::HtmlAnchorElement = documento
        .create_element("a")?
        .dyn_into()
        .map_err(|_| JsValue::from_str("elemento <a> inválido"))?;
    ancora.set_href(&url);
    ancora.set_download(nome);
    ancora.click();

    web_sys::Url::revoke_object_url(&url)
}

/// No-op no servidor (SSR): o download só ocorre no cliente.
#[cfg(not(target_arch = "wasm32"))]
pub fn baixar(_nome: &str, _conteudo: &str) {}
