//! `GET /pcp/produto/{codigo}/atividade` — apoio da tela de detalhe (doc 03 §4): status de
//! produção, histórico de produção e histórico de movimentação da linha de estoque. Só leitura;
//! os dados vêm do `bronze` (kardex e ordens do One).

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use pcp_db::atividade_produto as ativ;

use crate::erro::ApiError;
use crate::estado::AppState;

/// Quantas linhas de cada histórico devolver (a tela mostra as mais recentes).
const LIMITE_MOVIMENTOS: i64 = 60;
const LIMITE_PRODUCAO: i64 = 40;

#[derive(Serialize)]
pub struct StatusProducaoDto {
    pub ordens_abertas: i64,
    pub qtd_planejada: i64,
    pub em_producao: i64,
    pub aguardando: i64,
    pub planejado_em_producao: i64,
    pub produzido_em_producao: i64,
    pub finalizadas_recentes: i64,
}

#[derive(Serialize)]
pub struct MovimentoDto {
    pub data: String,
    pub tipo: String,
    pub quantidade: i64,
    pub saldo: i64,
}

#[derive(Serialize)]
pub struct OrdemProducaoDto {
    pub data: Option<String>,
    pub quantidade: i64,
    pub produzido: i64,
    pub status: Option<String>,
    pub lote: Option<i64>,
}

#[derive(Serialize)]
pub struct VendaMesDto {
    pub ano: i32,
    pub mes: i32,
    pub total: i64,
}

#[derive(Serialize)]
pub struct AtividadeDto {
    pub status_producao: StatusProducaoDto,
    pub producao: Vec<OrdemProducaoDto>,
    pub movimentos: Vec<MovimentoDto>,
    pub vendas_mensais: Vec<VendaMesDto>,
}

/// Atividade da linha de estoque (autenticado — qualquer papel lê).
///
/// # Errors
/// [`ApiError::Interno`] em falha de leitura.
pub async fn atividade(
    State(estado): State<AppState>,
    Path(codigo): Path<String>,
) -> Result<Json<AtividadeDto>, ApiError> {
    let recem = i32::try_from(estado.config().producao.recem_produzido_dias).unwrap_or(2);
    let status = ativ::status_producao(&estado.pool, &codigo, recem).await?;
    let producao = ativ::producao_historico(&estado.pool, &codigo, LIMITE_PRODUCAO).await?;
    let movimentos = ativ::movimentos(&estado.pool, &codigo, LIMITE_MOVIMENTOS).await?;
    let vendas_mensais = ativ::vendas_mensais(&estado.pool, &codigo).await?;
    Ok(Json(AtividadeDto {
        status_producao: StatusProducaoDto {
            ordens_abertas: status.ordens_abertas,
            qtd_planejada: status.qtd_planejada,
            em_producao: status.em_producao,
            aguardando: status.aguardando,
            planejado_em_producao: status.planejado_em_producao,
            produzido_em_producao: status.produzido_em_producao,
            finalizadas_recentes: status.finalizadas_recentes,
        },
        producao: producao
            .into_iter()
            .map(|o| OrdemProducaoDto {
                data: o.data.map(|d| d.to_string()),
                quantidade: i64::from(o.quantidade),
                produzido: i64::from(o.produzido),
                status: o.status,
                lote: o.lote,
            })
            .collect(),
        movimentos: movimentos
            .into_iter()
            .map(|m| MovimentoDto {
                data: m.data.to_string(),
                tipo: m.tipo,
                quantidade: i64::from(m.quantidade),
                saldo: m.saldo,
            })
            .collect(),
        vendas_mensais: vendas_mensais
            .into_iter()
            .map(|v| VendaMesDto {
                ano: v.ano,
                mes: v.mes,
                total: v.total,
            })
            .collect(),
    }))
}
