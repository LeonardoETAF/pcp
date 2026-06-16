//! Filtros salvos da Gestão de Estoque (doc 03 §3.2) — escrita do usuário, escopada ao dono.
//! Cada usuário só lê/escreve/apaga os seus (CLAUDE.md §7). O conteúdo do filtro é opaco aqui.

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use pcp_db::filtros_salvos;

use crate::erro::ApiError;
use crate::estado::AppState;
use crate::jwt::Claims;

#[derive(Serialize)]
pub struct FiltroSalvoDto {
    pub id: String,
    pub nome: String,
    pub filtro: serde_json::Value,
}

impl From<filtros_salvos::FiltroSalvo> for FiltroSalvoDto {
    fn from(f: filtros_salvos::FiltroSalvo) -> Self {
        Self {
            id: f.id.to_string(),
            nome: f.nome,
            filtro: f.filtro,
        }
    }
}

#[derive(Deserialize)]
pub struct NovoFiltroReq {
    pub nome: String,
    pub filtro: serde_json::Value,
}

/// `GET /pcp/estoque/filtros` — lista os filtros salvos do usuário.
///
/// # Errors
/// [`ApiError`] em sessão inválida ou falha de leitura.
pub async fn listar(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<FiltroSalvoDto>>, ApiError> {
    let itens = filtros_salvos::listar(&estado.pool, usuario_id(&claims)?).await?;
    Ok(Json(itens.into_iter().map(Into::into).collect()))
}

/// `POST /pcp/estoque/filtros` — salva (ou atualiza por nome) o filtro do usuário.
///
/// # Errors
/// [`ApiError::Requisicao`] se o nome for vazio; [`ApiError`] em falha de escrita.
pub async fn criar(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<NovoFiltroReq>,
) -> Result<Json<FiltroSalvoDto>, ApiError> {
    let nome = req.nome.trim();
    if nome.is_empty() {
        return Err(ApiError::Requisicao("nome do filtro vazio".to_owned()));
    }
    let salvo =
        filtros_salvos::salvar(&estado.pool, usuario_id(&claims)?, nome, &req.filtro).await?;
    Ok(Json(salvo.into()))
}

/// `DELETE /pcp/estoque/filtros/{id}` — remove um filtro do usuário (idempotente).
///
/// # Errors
/// [`ApiError`] em sessão inválida ou falha de escrita.
pub async fn excluir(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    filtros_salvos::excluir(&estado.pool, usuario_id(&claims)?, id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Extrai o id do usuário autenticado a partir das claims (sub = uuid).
fn usuario_id(claims: &Claims) -> Result<Uuid, ApiError> {
    Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Interno)
}
