//! `GET /pcp/produto/{codigo}/insights` — insights estatísticos do produto (doc 06 §3). Lê a
//! série de vendas (365 dias) e o contexto do `produto_ativo`; o cálculo é do motor PURO
//! `pcp-ai` (sem LLM, testável). A IA generativa (chat/análise) é à parte (4.2/4.3).

use axum::extract::{Path, State};
use axum::Json;
use chrono::Duration;

use pcp_ai::{analisar, ContextoProduto, Insights, PontoVenda};
use pcp_db::detalhe;

use crate::estado::AppState;
use sf_http::ApiError;

/// Janela de histórico para os insights (1 ano — sazonalidade e qualidade de dados, §3.3).
const JANELA_DIAS: i64 = 364;

/// Insights estatísticos do produto (autenticado — qualquer papel lê).
///
/// # Errors
/// [`ApiError::NaoEncontrado`] se o produto não existir; [`ApiError::Interno`] em falha de leitura.
pub async fn insights(
    State(estado): State<AppState>,
    Path(codigo): Path<String>,
) -> Result<Json<Insights>, ApiError> {
    let d = detalhe::produto(&estado.pool, &codigo)
        .await?
        .ok_or(ApiError::NaoEncontrado)?;
    let fim = d.dt_ref;
    let inicio = fim - Duration::days(JANELA_DIAS);
    let serie = detalhe::vendas_intervalo(&estado.pool, &codigo, inicio, fim).await?;

    #[allow(clippy::cast_precision_loss)] // quantidades pequenas: conversão exata
    let pontos: Vec<PontoVenda> = serie
        .into_iter()
        .map(|p| PontoVenda {
            data: p.data,
            qtd: p.valor as f64,
        })
        .collect();
    let ctx = ContextoProduto {
        cobertura_dias: d.cobertura_dias,
        qtd_disponivel: d.qtd_disponivel,
        estoque_recomendado: d.estoque_total_recomendado,
    };
    Ok(Json(analisar(&pontos, inicio, fim, &ctx)))
}
