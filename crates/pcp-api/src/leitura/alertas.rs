//! `GET /pcp/alertas` — alertas do dia para a Central de Alertas (doc 04 §6.2).
//! Lê `alerta` (do dia mais recente) enriquecido com `produto_ativo`.

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use pcp_db::leituras::{self, AlertaCompleto};

use crate::erro::ApiError;
use crate::estado::AppState;

#[derive(Serialize)]
pub struct AlertaDto {
    pub codigo_estoque: String,
    pub prioridade: String,
    pub classe: String,
    pub qtd_sugerida: i64,
    pub cobertura_dias: f64,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub status: Option<String>,
}

impl From<AlertaCompleto> for AlertaDto {
    fn from(a: AlertaCompleto) -> Self {
        Self {
            codigo_estoque: a.codigo_estoque,
            prioridade: a.prioridade,
            classe: a.classe,
            qtd_sugerida: a.qtd_sugerida,
            cobertura_dias: a.cobertura_dias,
            produto: a.produto,
            configuracao: a.configuracao,
            status: a.status,
        }
    }
}

/// Alertas do dia (autenticado — qualquer papel lê).
///
/// # Errors
/// [`ApiError::Interno`] em falha de leitura.
pub async fn alertas(State(estado): State<AppState>) -> Result<Json<Vec<AlertaDto>>, ApiError> {
    let linhas = leituras::alertas_do_dia(&estado.pool).await?;
    Ok(Json(linhas.into_iter().map(Into::into).collect()))
}
