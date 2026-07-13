//! `GET /pcp/dashboard/classes` — painel de metas físicas ABC (doc 02 §9.1) e cobertura média
//! por classe (doc 03 §2). Metas vêm da config (fonte única, §3.7); a API só lê e compara.

use axum::extract::State;
use axum::Json;
use serde::Serialize;

use pcp_db::leituras::{self, ResumoClasse};

use crate::estado::AppState;
use sf_http::ApiError;

/// Tolerância da meta física: |real − meta| ≤ 3 p.p. é "atingida" (doc 02 §9.1).
const TOLERANCIA_META_PP: f64 = 3.0;

#[derive(Serialize)]
pub struct ClasseDto {
    pub classe: String,
    pub qtd_produtos: i64,
    pub estoque_fisico: i64,
    pub pct_fisico_real: f64,
    pub pct_fisico_meta: Option<u32>,
    pub meta_atingida: Option<bool>,
    pub cobertura_media: Option<f64>,
    /// Meta de cobertura da classe em dias (config §11) — base do anel de cobertura do dashboard.
    pub cobertura_meta_dias: u32,
}

/// Resumo por classe com metas físicas e cobertura (autenticado — qualquer papel lê).
///
/// # Errors
/// [`ApiError::Interno`] em falha de leitura.
pub async fn classes(State(estado): State<AppState>) -> Result<Json<Vec<ClasseDto>>, ApiError> {
    let resumo = leituras::resumo_por_classe(&estado.pool).await?;
    let total_fisico: i64 = resumo.iter().map(|r| r.estoque_fisico).sum();
    let config = estado.config();
    let dtos = resumo
        .into_iter()
        .map(|r| dto(r, total_fisico, &config))
        .collect();
    Ok(Json(dtos))
}

#[allow(clippy::cast_precision_loss)] // estoques pequenos: conversão exata para f64
fn dto(r: ResumoClasse, total_fisico: i64, config: &pcp_config::Config) -> ClasseDto {
    let pct_real = if total_fisico > 0 {
        (r.estoque_fisico as f64 / total_fisico as f64) * 100.0
    } else {
        0.0
    };
    let meta = meta_fisica(config, &r.classe);
    let atingida = meta.map(|m| (pct_real - f64::from(m)).abs() <= TOLERANCIA_META_PP);
    let cobertura_meta_dias = meta_cobertura(config, &r.classe);
    ClasseDto {
        classe: r.classe,
        qtd_produtos: r.qtd_produtos,
        estoque_fisico: r.estoque_fisico,
        pct_fisico_real: (pct_real * 10.0).round() / 10.0,
        pct_fisico_meta: meta,
        meta_atingida: atingida,
        cobertura_media: r.cobertura_media.map(|c| (c * 10.0).round() / 10.0),
        cobertura_meta_dias,
    }
}

/// Meta de participação no estoque físico da classe (doc 02 §9.1). Só A/B/C/D têm meta.
fn meta_fisica(c: &pcp_config::Config, classe: &str) -> Option<u32> {
    let m = &c.metas_estoque_fisico_pct;
    match classe {
        "A" => Some(m.a),
        "B" => Some(m.b),
        "C" => Some(m.c),
        "D" => Some(m.d),
        _ => None,
    }
}

/// Meta de cobertura (dias) da classe (config §11). `default` para classes sem meta própria.
fn meta_cobertura(c: &pcp_config::Config, classe: &str) -> u32 {
    let m = &c.metas_cobertura_dias;
    match classe {
        "A" => m.a,
        "B" => m.b,
        "C" => m.c,
        "D" => m.d,
        "F" => m.f,
        "N" => m.n,
        _ => m.default,
    }
}
