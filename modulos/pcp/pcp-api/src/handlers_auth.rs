//! Endpoints públicos de autenticação: login, refresh e logout.

use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use sf_db::{refresh_tokens, usuarios};

use crate::estado::AppState;
use sf_auth::{gerar_access, verificar};
use sf_auth::{gerar_refresh, hash_refresh};
use sf_http::ApiError;

#[derive(Deserialize)]
pub struct LoginReq {
    pub email: String,
    pub senha: String,
}

#[derive(Serialize)]
pub struct TokensResp {
    pub access_token: String,
    pub refresh_token: String,
    pub papel: String,
}

#[derive(Deserialize)]
pub struct RefreshReq {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct AccessResp {
    pub access_token: String,
}

/// `POST /auth/login` — valida credenciais e emite access + refresh token.
///
/// # Errors
/// [`ApiError::CredenciaisInvalidas`] se e-mail/senha não baterem ou o usuário estiver inativo.
pub async fn login(
    State(estado): State<AppState>,
    Json(req): Json<LoginReq>,
) -> Result<Json<TokensResp>, ApiError> {
    let email = req.email.trim().to_lowercase();
    let usuario = usuarios::buscar_por_email(&estado.pool, &email)
        .await?
        .filter(|u| u.ativo)
        .ok_or(ApiError::CredenciaisInvalidas)?;
    if !verificar(&req.senha, &usuario.senha_hash) {
        return Err(ApiError::CredenciaisInvalidas);
    }
    let access = gerar_access(
        &usuario.id.to_string(),
        &usuario.papel,
        &estado.jwt_secret,
        estado.access_ttl,
    )?;
    let (bruto, hash) = gerar_refresh();
    refresh_tokens::criar(
        &estado.pool,
        usuario.id,
        &hash,
        Utc::now() + estado.refresh_ttl,
    )
    .await?;
    Ok(Json(TokensResp {
        access_token: access,
        refresh_token: bruto,
        papel: usuario.papel,
    }))
}

/// `POST /auth/refresh` — troca um refresh token válido por um novo access token.
///
/// # Errors
/// [`ApiError::NaoAutenticado`] se o refresh token for inválido/expirado/revogado.
pub async fn refresh(
    State(estado): State<AppState>,
    Json(req): Json<RefreshReq>,
) -> Result<Json<AccessResp>, ApiError> {
    let token = refresh_tokens::buscar_valido(&estado.pool, &hash_refresh(&req.refresh_token))
        .await?
        .ok_or(ApiError::NaoAutenticado)?;
    let usuario = usuarios::buscar_por_id(&estado.pool, token.usuario_id)
        .await?
        .filter(|u| u.ativo)
        .ok_or(ApiError::NaoAutenticado)?;
    let access = gerar_access(
        &usuario.id.to_string(),
        &usuario.papel,
        &estado.jwt_secret,
        estado.access_ttl,
    )?;
    Ok(Json(AccessResp {
        access_token: access,
    }))
}

/// `POST /auth/logout` — revoga o refresh token informado (idempotente).
///
/// # Errors
/// [`ApiError::Interno`] em falha de banco.
pub async fn logout(
    State(estado): State<AppState>,
    Json(req): Json<RefreshReq>,
) -> Result<StatusCode, ApiError> {
    refresh_tokens::revogar(&estado.pool, &hash_refresh(&req.refresh_token)).await?;
    Ok(StatusCode::NO_CONTENT)
}
