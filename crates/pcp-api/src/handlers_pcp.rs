//! Endpoints protegidos sob `/pcp` (exigem autenticação; alguns exigem papel mínimo).

use axum::extract::State;
use axum::http::StatusCode;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use pcp_db::usuarios;

use crate::erro::ApiError;
use crate::estado::AppState;
use crate::jwt::Claims;
use crate::papel::Papel;
use crate::senha;

/// `GET /pcp/me` — dados do usuário autenticado (qualquer papel).
#[allow(clippy::unused_async)] // handler assíncrono exigido pelo Axum
pub async fn me(Extension(claims): Extension<Claims>) -> Json<Value> {
    Json(json!({ "id": claims.sub, "papel": claims.papel }))
}

/// `GET /pcp/aprovacoes` — área de aprovações (gestor ou admin — CLAUDE.md §7.3).
///
/// # Errors
/// [`ApiError::Proibido`] se o papel for inferior a gestor.
#[allow(clippy::unused_async)] // handler assíncrono exigido pelo Axum
pub async fn area_aprovacoes(
    Extension(claims): Extension<Claims>,
) -> Result<Json<Value>, ApiError> {
    claims.exige(Papel::Gestor)?;
    Ok(Json(json!({ "area": "aprovacoes", "acesso": "ok" })))
}

#[derive(Deserialize)]
pub struct NovoUsuarioReq {
    pub email: String,
    pub senha: String,
    pub papel: String,
    pub nome: Option<String>,
}

#[derive(Serialize)]
pub struct UsuarioResp {
    pub id: String,
    pub email: String,
    pub papel: String,
    pub nome: Option<String>,
}

/// `POST /pcp/usuarios` — cria um usuário (somente admin — CLAUDE.md §7.3).
///
/// # Errors
/// [`ApiError::Proibido`] (não admin), [`ApiError::Requisicao`] (dados inválidos),
/// [`ApiError::Conflito`] (e-mail já cadastrado).
pub async fn criar_usuario(
    Extension(claims): Extension<Claims>,
    State(estado): State<AppState>,
    Json(req): Json<NovoUsuarioReq>,
) -> Result<(StatusCode, Json<UsuarioResp>), ApiError> {
    claims.exige(Papel::Admin)?;
    let papel = Papel::tentar_de(&req.papel)
        .ok_or_else(|| ApiError::Requisicao("papel inválido (use analista|gestor|admin)".into()))?;
    if req.senha.len() < 8 {
        return Err(ApiError::Requisicao(
            "senha muito curta (mínimo 8 caracteres)".into(),
        ));
    }
    let email = req.email.trim().to_lowercase();
    if email.is_empty() {
        return Err(ApiError::Requisicao("e-mail obrigatório".into()));
    }
    if usuarios::buscar_por_email(&estado.pool, &email)
        .await?
        .is_some()
    {
        return Err(ApiError::Conflito("e-mail já cadastrado".into()));
    }
    let hash = senha::hashear(&req.senha)?;
    let usuario = usuarios::criar(
        &estado.pool,
        &email,
        &hash,
        papel.como_str(),
        req.nome.as_deref(),
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(UsuarioResp {
            id: usuario.id.to_string(),
            email: usuario.email,
            papel: usuario.papel,
            nome: usuario.nome,
        }),
    ))
}
