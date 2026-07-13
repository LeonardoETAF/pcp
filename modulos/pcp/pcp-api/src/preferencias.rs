//! Preferências de exibição do usuário (doc 03 §8). Cada usuário lê/grava as suas (autenticado).

use axum::extract::State;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use pcp_db::preferencias::{self, Preferencia};

use crate::estado::AppState;
use sf_auth::Claims;
use sf_http::ApiError;

/// Páginas iniciais permitidas (rotas existentes).
const PAGINAS: [&str; 4] = ["dashboard", "estoque", "alertas", "abc"];
/// Tamanhos de página permitidos (espelha a tabela de estoque — doc 03 §3.4).
const TAMANHOS: [i32; 4] = [50, 100, 500, 1000];

#[derive(Serialize, Deserialize)]
pub struct PreferenciaDto {
    pub pagina_inicial: String,
    pub tamanho_pagina: i32,
}

impl From<Preferencia> for PreferenciaDto {
    fn from(p: Preferencia) -> Self {
        Self {
            pagina_inicial: p.pagina_inicial,
            tamanho_pagina: p.tamanho_pagina,
        }
    }
}

/// `GET /pcp/preferencias` — preferências do usuário autenticado (default se não houver).
///
/// # Errors
/// [`ApiError`] em sessão inválida ou falha de leitura.
pub async fn obter(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<PreferenciaDto>, ApiError> {
    let p = preferencias::obter(&estado.pool, usuario_id(&claims)?).await?;
    Ok(Json(p.into()))
}

/// `PUT /pcp/preferencias` — grava as preferências do usuário autenticado.
///
/// # Errors
/// [`ApiError::Requisicao`] (valores fora da allowlist); [`ApiError`] em falha de escrita.
pub async fn salvar(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<PreferenciaDto>,
) -> Result<Json<PreferenciaDto>, ApiError> {
    if !PAGINAS.contains(&req.pagina_inicial.as_str()) {
        return Err(ApiError::Requisicao("página inicial inválida".to_owned()));
    }
    if !TAMANHOS.contains(&req.tamanho_pagina) {
        return Err(ApiError::Requisicao(
            "tamanho de página inválido".to_owned(),
        ));
    }
    preferencias::salvar(
        &estado.pool,
        usuario_id(&claims)?,
        &req.pagina_inicial,
        req.tamanho_pagina,
    )
    .await?;
    Ok(Json(req))
}

fn usuario_id(claims: &Claims) -> Result<Uuid, ApiError> {
    Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Interno)
}
