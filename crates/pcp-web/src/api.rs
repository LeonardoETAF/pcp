//! Ponte com a `pcp-api` via *server functions* do Leptos: o corpo roda no servidor SSR
//! (server-to-server), evitando CORS e mantendo segredos fora do WASM. Frontend burro: só
//! repassa credenciais e devolve o token; nenhuma regra de negócio aqui (CLAUDE.md §3/§7).

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Contagem `(rótulo, quantidade)` das distribuições do painel.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Contagem {
    pub rotulo: String,
    pub quantidade: i64,
}

/// Métricas agregadas do painel (`GET /pcp/dashboard`). Valores já calculados pelo motor.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PainelResumo {
    pub data_ref: Option<String>,
    pub total_produtos: i64,
    pub total_sugerido: i64,
    pub cobertura_media: Option<f64>,
    pub por_classe: Vec<Contagem>,
    pub por_status: Vec<Contagem>,
}

/// Linha de produto da tabela de estoque (`GET /pcp/estoque`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinhaEstoque {
    pub codigo_estoque: String,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub classe: String,
    pub qtd_disponivel: i64,
    pub cobertura_dias: f64,
    pub estoque_total_recomendado: i64,
    pub status: String,
    pub qtd_sugerida: i64,
    pub fora_de_linha: bool,
}

/// Página de produtos (ignora `limite`/`deslocamento` do payload — só precisamos de itens/total).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PaginaEstoque {
    pub itens: Vec<LinhaEstoque>,
    pub total: i64,
}

/// Métricas do painel (`GET /pcp/dashboard`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = Painel, prefix = "/api")]
pub async fn painel(token: String) -> Result<PainelResumo, ServerFnError> {
    obter_json("/pcp/dashboard", &token).await
}

/// Produtos ativos paginados (`GET /pcp/estoque`), com filtro opcional de status.
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = ListarEstoque, prefix = "/api")]
pub async fn estoque(
    token: String,
    status: Option<String>,
    limite: i64,
) -> Result<PaginaEstoque, ServerFnError> {
    let mut caminho = format!("/pcp/estoque?limite={limite}");
    if let Some(s) = status.filter(|s| !s.is_empty()) {
        caminho.push_str("&status=");
        caminho.push_str(&s);
    }
    obter_json(&caminho, &token).await
}

/// Papel do usuário autenticado (`GET /pcp/me`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede ou sessão inválida.
#[server(name = Perfil, prefix = "/api")]
pub async fn perfil(token: String) -> Result<String, ServerFnError> {
    #[derive(Deserialize)]
    struct Me {
        papel: String,
    }
    let me: Me = obter_json("/pcp/me", &token).await?;
    Ok(me.papel)
}

/// Helper (só servidor): GET autenticado na `pcp-api` e desserialização do JSON.
#[cfg(feature = "ssr")]
async fn obter_json<T: serde::de::DeserializeOwned>(
    caminho: &str,
    token: &str,
) -> Result<T, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .get(format!("{base}{caminho}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao carregar dados"));
    }
    resposta
        .json::<T>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Alerta de produção como entregue pela API (`GET /pcp/alertas`). Valores já calculados pelo
/// motor — o frontend só exibe (CLAUDE.md §3). Espelha o DTO da `pcp-api`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertaResumo {
    pub codigo_estoque: String,
    pub prioridade: String,
    pub classe: String,
    pub qtd_sugerida: i64,
    pub cobertura_dias: f64,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub status: Option<String>,
}

/// Lista os alertas do dia (`GET /pcp/alertas`) com o token do usuário (Bearer).
///
/// # Errors
/// [`ServerFnError`] se a API não responder, a sessão expirar ou o corpo for inválido.
#[server(name = ListarAlertas, prefix = "/api")]
pub async fn alertas(token: String) -> Result<Vec<AlertaResumo>, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .get(format!("{base}/pcp/alertas"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao carregar os alertas"));
    }
    resposta
        .json::<Vec<AlertaResumo>>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

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
