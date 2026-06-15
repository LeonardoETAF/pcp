//! `GET /pcp/estoque` — tabela de produtos ativos paginada no servidor (doc 04 §6.2 / §15).
//! Filtros opcionais `classe`/`status`; só lê `produto_ativo`.

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use pcp_db::leituras::{self, LinhaEstoque};

use crate::erro::ApiError;
use crate::estado::AppState;

/// Tamanho de página padrão e teto (paginação no servidor — §15).
const LIMITE_PADRAO: i64 = 50;
const LIMITE_MAX: i64 = 200;

#[derive(Deserialize)]
pub struct ParamsEstoque {
    pub classe: Option<String>,
    pub status: Option<String>,
    pub limite: Option<i64>,
    pub deslocamento: Option<i64>,
}

#[derive(Serialize)]
pub struct LinhaEstoqueDto {
    pub codigo_estoque: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub classe: String,
    pub qtd_disponivel: i64,
    pub cobertura_dias: f64,
    pub estoque_total_recomendado: i64,
    pub status: String,
    pub qtd_sugerida: i64,
    pub fora_de_linha: bool,
}

impl From<LinhaEstoque> for LinhaEstoqueDto {
    fn from(l: LinhaEstoque) -> Self {
        Self {
            codigo_estoque: l.codigo_estoque,
            sku: l.sku,
            produto: l.produto,
            configuracao: l.configuracao,
            classe: l.classe,
            qtd_disponivel: l.qtd_disponivel,
            cobertura_dias: l.cobertura_dias,
            estoque_total_recomendado: l.estoque_total_recomendado,
            status: l.status,
            qtd_sugerida: l.qtd_sugerida,
            fora_de_linha: l.fora_de_linha,
        }
    }
}

#[derive(Serialize)]
pub struct PaginaEstoqueDto {
    pub itens: Vec<LinhaEstoqueDto>,
    pub total: i64,
    pub limite: i64,
    pub deslocamento: i64,
}

/// Produtos ativos paginados (autenticado — qualquer papel lê).
///
/// # Errors
/// [`ApiError::Interno`] em falha de leitura.
pub async fn estoque(
    State(estado): State<AppState>,
    Query(params): Query<ParamsEstoque>,
) -> Result<Json<PaginaEstoqueDto>, ApiError> {
    let limite = params.limite.unwrap_or(LIMITE_PADRAO).clamp(1, LIMITE_MAX);
    let deslocamento = params.deslocamento.unwrap_or(0).max(0);
    let pagina = leituras::produtos_paginado(
        &estado.pool,
        params.classe.as_deref(),
        params.status.as_deref(),
        limite,
        deslocamento,
    )
    .await?;
    Ok(Json(PaginaEstoqueDto {
        itens: pagina.itens.into_iter().map(Into::into).collect(),
        total: pagina.total,
        limite,
        deslocamento,
    }))
}
