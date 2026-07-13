//! `GET /pcp/dashboard` — métricas agregadas da home (doc 04 §6.2). Só lê `produto_ativo`.

use axum::extract::State;
use axum::Json;
use chrono::NaiveDate;
use serde::Serialize;

use pcp_db::leituras::{self, Contagem};

use crate::estado::AppState;
use sf_http::ApiError;

#[derive(Serialize)]
pub struct ContagemDto {
    pub rotulo: String,
    pub quantidade: i64,
}

impl From<Contagem> for ContagemDto {
    fn from(c: Contagem) -> Self {
        Self {
            rotulo: c.rotulo,
            quantidade: c.quantidade,
        }
    }
}

#[derive(Serialize)]
pub struct DashboardDto {
    pub data_ref: Option<NaiveDate>,
    pub total_produtos: i64,
    pub total_sugerido: i64,
    pub cobertura_media: Option<f64>,
    pub por_classe: Vec<ContagemDto>,
    pub por_status: Vec<ContagemDto>,
}

/// Métricas do dashboard (autenticado — qualquer papel lê; CLAUDE.md §7.1/§7.3).
///
/// # Errors
/// [`ApiError::Interno`] em falha de leitura.
pub async fn dashboard(State(estado): State<AppState>) -> Result<Json<DashboardDto>, ApiError> {
    let r = leituras::dashboard(&estado.pool).await?;
    Ok(Json(DashboardDto {
        data_ref: r.data_ref,
        total_produtos: r.total_produtos,
        total_sugerido: r.total_sugerido,
        cobertura_media: r.cobertura_media,
        por_classe: r.por_classe.into_iter().map(Into::into).collect(),
        por_status: r.por_status.into_iter().map(Into::into).collect(),
    }))
}

/// Quantos meses a série mensal de vendas devolve (gráfico do dashboard).
const MESES_SERIE: i64 = 6;

#[derive(Serialize)]
pub struct VendaMesDto {
    pub ano: i32,
    pub mes: i32,
    pub total: i64,
}

/// `GET /pcp/dashboard/vendas-mensais` — série mensal de vendas (dado real de `vendas_dia`),
/// últimos 6 meses em ordem cronológica (autenticado — qualquer papel lê).
///
/// # Errors
/// [`ApiError::Interno`] em falha de leitura.
pub async fn vendas_mensais(
    State(estado): State<AppState>,
) -> Result<Json<Vec<VendaMesDto>>, ApiError> {
    let series = leituras::vendas_mensais(&estado.pool, MESES_SERIE).await?;
    Ok(Json(
        series
            .into_iter()
            .map(|v| VendaMesDto {
                ano: v.ano,
                mes: v.mes,
                total: v.total,
            })
            .collect(),
    ))
}
