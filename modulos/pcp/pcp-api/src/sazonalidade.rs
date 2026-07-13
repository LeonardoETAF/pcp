//! Fatores sazonais (doc 03 §8 / doc 02 §4): visualização dos vigentes (qualquer autenticado) e
//! override manual pelo GESTOR com justificativa e auditoria (§7.5). O clamp vem da config.

use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use pcp_db::sazonalidade;

use crate::estado::AppState;
use sf_auth::Claims;
use sf_auth::Papel;
use sf_http::ApiError;

#[derive(Serialize)]
pub struct FatorMesDto {
    pub mes: i16,
    pub fator: f64,
}

#[derive(Serialize)]
pub struct SazonalAuditoriaDto {
    pub mes: i16,
    pub fator_anterior: Option<f64>,
    pub fator_novo: f64,
    pub justificativa: Option<String>,
    pub por_id: String,
    pub em: String,
}

#[derive(Deserialize)]
pub struct OverrideReq {
    pub fator: f64,
    pub justificativa: Option<String>,
}

/// `GET /pcp/sazonalidade` — fatores vigentes por mês (autenticado).
///
/// # Errors
/// [`ApiError`] em falha de leitura.
pub async fn listar(State(estado): State<AppState>) -> Result<Json<Vec<FatorMesDto>>, ApiError> {
    let itens = sazonalidade::listar(&estado.pool).await?;
    Ok(Json(
        itens
            .into_iter()
            .map(|(mes, fator)| FatorMesDto { mes, fator })
            .collect(),
    ))
}

/// `PUT /pcp/sazonalidade/{mes}` — override manual do fator do mês (gestor), dentro do clamp.
///
/// # Errors
/// [`ApiError::Proibido`] (não gestor); [`ApiError::Requisicao`] (mês/fator inválido).
pub async fn override_mes(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(mes): Path<i16>,
    Json(req): Json<OverrideReq>,
) -> Result<Json<FatorMesDto>, ApiError> {
    claims.exige(Papel::Gestor)?;
    if !(1..=12).contains(&mes) {
        return Err(ApiError::Requisicao("mês inválido".to_owned()));
    }
    let s = estado.config().sazonalidade;
    if req.fator < s.clamp_min || req.fator > s.clamp_max {
        return Err(ApiError::Requisicao(format!(
            "fator fora do intervalo permitido ({}–{})",
            s.clamp_min, s.clamp_max
        )));
    }
    let por_id = Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Interno)?;
    sazonalidade::override_mes(
        &estado.pool,
        mes,
        req.fator,
        req.justificativa.as_deref(),
        por_id,
    )
    .await?;
    Ok(Json(FatorMesDto {
        mes,
        fator: req.fator,
    }))
}

/// `GET /pcp/sazonalidade/auditoria` — trilha de overrides (gestor).
///
/// # Errors
/// [`ApiError::Proibido`] (não gestor); [`ApiError`] em falha de leitura.
pub async fn auditoria(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<SazonalAuditoriaDto>>, ApiError> {
    claims.exige(Papel::Gestor)?;
    let itens = sazonalidade::auditoria(&estado.pool, 50).await?;
    Ok(Json(
        itens
            .into_iter()
            .map(|e| SazonalAuditoriaDto {
                mes: e.mes,
                fator_anterior: e.fator_anterior,
                fator_novo: e.fator_novo,
                justificativa: e.justificativa,
                por_id: e.por_id.to_string(),
                em: e.em.to_rfc3339(),
            })
            .collect(),
    ))
}
