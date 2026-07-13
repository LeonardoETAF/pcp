//! Workflow de fora de linha / ciclo de vida (doc 03 §6). Fila de sugestões abertas (qualquer
//! autenticado lê) e transição de estado pelo GESTOR, com auditoria inline (§7.5). A máquina de
//! estados é validada no `pcp-core` (regra única, §3.1).

use axum::extract::{Path, State};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use pcp_core::ciclo_vida::{transicionar, EstadoCicloVida};
use pcp_db::ciclo_vida::{self, SugestaoCicloVida};

use crate::erro::ApiError;
use crate::estado::AppState;
use crate::jwt::Claims;
use crate::papel::Papel;

#[derive(Serialize)]
pub struct SugestaoDto {
    pub id: String,
    pub codigo_estoque: String,
    pub acao_sugerida: String,
    pub pontuacao: i16,
    pub nivel_certeza: String,
    pub criterios: Vec<String>,
    pub estado: String,
    pub data_analise: String,
    pub aplicado_por: Option<String>,
    pub observacoes: Option<String>,
}

impl From<SugestaoCicloVida> for SugestaoDto {
    fn from(s: SugestaoCicloVida) -> Self {
        Self {
            id: s.id.to_string(),
            codigo_estoque: s.codigo_estoque,
            acao_sugerida: s.acao_sugerida,
            pontuacao: s.pontuacao,
            nivel_certeza: s.nivel_certeza,
            criterios: s.criterios,
            estado: s.estado,
            data_analise: s.data_analise.to_string(),
            aplicado_por: s.aplicado_por,
            observacoes: s.observacoes,
        }
    }
}

#[derive(Deserialize)]
pub struct TransicaoReq {
    pub para_estado: String,
    pub observacao: Option<String>,
}

/// `GET /pcp/ciclo-vida` — fila de sugestões abertas (autenticado).
///
/// # Errors
/// [`ApiError`] em falha de leitura.
pub async fn fila(State(estado): State<AppState>) -> Result<Json<Vec<SugestaoDto>>, ApiError> {
    let itens = ciclo_vida::listar_abertas(&estado.pool).await?;
    Ok(Json(itens.into_iter().map(Into::into).collect()))
}

/// `POST /pcp/ciclo-vida/{id}/transicao` — analisar/aplicar/recusar (gestor+). Valida a máquina de
/// estados no `pcp-core` e registra quem agiu (auditoria §7.5).
///
/// # Errors
/// [`ApiError::Proibido`] (não gestor); [`ApiError::Requisicao`] (transição inválida);
/// [`ApiError::NaoEncontrado`] (id inexistente).
pub async fn transicionar_estado(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<TransicaoReq>,
) -> Result<Json<SugestaoDto>, ApiError> {
    claims.exige(Papel::Gestor)?;
    let para = EstadoCicloVida::tentar_de(&req.para_estado)
        .ok_or_else(|| ApiError::Requisicao("estado de destino inválido".to_owned()))?;
    let atual = ciclo_vida::estado_atual(&estado.pool, id)
        .await?
        .ok_or(ApiError::NaoEncontrado)?;
    let de = EstadoCicloVida::tentar_de(&atual).ok_or(ApiError::Interno)?;
    transicionar(de, para).map_err(|e| ApiError::Requisicao(e.to_string()))?;

    let s = ciclo_vida::transicionar(
        &estado.pool,
        id,
        para.codigo(),
        &claims.sub,
        req.observacao.as_deref(),
    )
    .await?;
    Ok(Json(s.into()))
}
