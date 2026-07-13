//! `GET /pcp/abc/tabela` — tabela da Classificação ABC (doc 03 §6): 1 linha por produto pela
//! classificação mais recente. Só lê; o Pareto/percentual já vêm calculados pelo motor (§3).

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use pcp_db::leituras::{self, LinhaAbc};

use crate::estado::AppState;
use sf_http::ApiError;

#[derive(Serialize)]
pub struct LinhaAbcDto {
    pub codigo_estoque: String,
    pub produto: Option<String>,
    pub classe: String,
    pub volume_janela: i64,
    pub percentual_acumulado: Option<f64>,
    pub fator_estoque: f64,
    pub estoque_atual: i64,
    pub status: String,
}

impl From<LinhaAbc> for LinhaAbcDto {
    fn from(l: LinhaAbc) -> Self {
        Self {
            codigo_estoque: l.codigo_estoque,
            produto: l.produto,
            classe: l.classe,
            volume_janela: l.volume_janela,
            percentual_acumulado: l.percentual_acumulado,
            fator_estoque: l.fator_estoque,
            estoque_atual: l.estoque_atual,
            status: l.status,
        }
    }
}

/// Tabela ABC completa (autenticado — qualquer papel lê).
///
/// # Errors
/// [`ApiError::Interno`] em falha de leitura.
pub async fn abc_tabela(
    State(estado): State<AppState>,
) -> Result<Json<Vec<LinhaAbcDto>>, ApiError> {
    let linhas = leituras::classificacao_recente(&estado.pool).await?;
    Ok(Json(linhas.into_iter().map(Into::into).collect()))
}
