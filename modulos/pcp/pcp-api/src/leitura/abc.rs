//! `GET /pcp/abc` — distribuição por classe ABC (doc 04 §6.2). Só lê `produto_ativo`.

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use pcp_db::leituras::{self, DistribuicaoClasse};

use crate::estado::AppState;
use sf_http::ApiError;

#[derive(Serialize)]
pub struct DistribuicaoDto {
    pub classe: String,
    pub quantidade: i64,
    pub volume: i64,
    pub recomendado: i64,
}

impl From<DistribuicaoClasse> for DistribuicaoDto {
    fn from(d: DistribuicaoClasse) -> Self {
        Self {
            classe: d.classe,
            quantidade: d.quantidade,
            volume: d.volume,
            recomendado: d.recomendado,
        }
    }
}

/// Distribuição por classe ABC (autenticado — qualquer papel lê).
///
/// # Errors
/// [`ApiError::Interno`] em falha de leitura.
pub async fn abc(State(estado): State<AppState>) -> Result<Json<Vec<DistribuicaoDto>>, ApiError> {
    let linhas = leituras::distribuicao_abc(&estado.pool).await?;
    Ok(Json(linhas.into_iter().map(Into::into).collect()))
}
