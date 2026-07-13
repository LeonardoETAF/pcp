//! Gestão de usuários (doc 03 §8) — somente ADMIN (CLAUDE.md §7.3). Listagem e atualização de
//! papel/situação. A criação fica em `handlers_pcp::criar_usuario`. `senha_hash` nunca é exposto.

use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use sf_db::usuarios::{self, Usuario};

use crate::estado::AppState;
use sf_auth::Claims;
use sf_auth::Papel;
use sf_http::ApiError;

#[derive(Serialize)]
pub struct UsuarioDto {
    pub id: String,
    pub email: String,
    pub papel: String,
    pub nome: Option<String>,
    pub ativo: bool,
}

impl From<Usuario> for UsuarioDto {
    fn from(u: Usuario) -> Self {
        Self {
            id: u.id.to_string(),
            email: u.email,
            papel: u.papel,
            nome: u.nome,
            ativo: u.ativo,
        }
    }
}

#[derive(Deserialize)]
pub struct AtualizarUsuarioReq {
    pub papel: String,
    pub ativo: bool,
}

/// `GET /pcp/usuarios` — lista os usuários (admin).
///
/// # Errors
/// [`ApiError::Proibido`] (não admin); [`ApiError`] em falha de leitura.
pub async fn listar(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<UsuarioDto>>, ApiError> {
    claims.exige(Papel::Admin)?;
    let itens = usuarios::listar(&estado.pool).await?;
    Ok(Json(itens.into_iter().map(Into::into).collect()))
}

/// `PUT /pcp/usuarios/{id}` — atualiza papel/situação (admin).
///
/// # Errors
/// [`ApiError::Proibido`] (não admin); [`ApiError::Requisicao`] (papel inválido);
/// [`ApiError::NaoEncontrado`] (id inexistente).
pub async fn atualizar(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<AtualizarUsuarioReq>,
) -> Result<Json<UsuarioDto>, ApiError> {
    claims.exige(Papel::Admin)?;
    let papel = Papel::tentar_de(&req.papel)
        .ok_or_else(|| ApiError::Requisicao("papel inválido".to_owned()))?;
    let u = usuarios::atualizar(&estado.pool, id, papel.como_str(), req.ativo)
        .await?
        .ok_or(ApiError::NaoEncontrado)?;
    Ok(Json(u.into()))
}
