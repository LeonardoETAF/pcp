//! Ponte com a `pcp-api` via *server functions* do Leptos: o corpo roda no servidor SSR
//! (server-to-server), evitando CORS e mantendo segredos fora do WASM. Frontend burro: só
//! repassa credenciais e devolve o token; nenhuma regra de negócio aqui (CLAUDE.md §3/§7).

use leptos::prelude::*;

/// Autentica na `pcp-api` (`POST /auth/login`) e devolve o `access_token`.
///
/// # Errors
/// [`ServerFnError`] se a API não responder ou as credenciais forem inválidas.
#[server(name = Login, prefix = "/api")]
pub async fn login(email: String, senha: String) -> Result<String, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .post(format!("{base}/auth/login"))
        .json(&serde_json::json!({ "email": email, "senha": senha }))
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("credenciais inválidas"));
    }
    let corpo: serde_json::Value = resposta
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    corpo["access_token"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| ServerFnError::new("resposta da API sem access_token"))
}
