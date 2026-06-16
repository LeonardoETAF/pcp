//! `GET /pcp/produto/{codigo}` — Detalhe do Produto (doc 03 §4). Entrega cabeçalho, regra da
//! classe (metas/limiar/fator vindos da config — fonte única, §3.7), métricas e séries de 90
//! dias. Tudo já calculado pelo motor; a API só lê e descreve — não recalcula regra (§3.2).

use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;

use pcp_config::Config;
use pcp_db::detalhe::{self, DetalheProduto, PontoSerie};

use crate::erro::ApiError;
use crate::estado::AppState;

#[derive(Serialize)]
pub struct RegraClasseDto {
    pub meta_cobertura_dias: u32,
    pub limiar_critico_dias: Option<u32>,
    pub fator_estoque: f64,
    pub justificativa: String,
}

#[derive(Serialize)]
pub struct MetricasDto {
    pub qtd_estoque: i64,
    pub qtd_reserva: i64,
    pub qtd_disponivel: i64,
    pub cobertura_dias: f64,
    pub media_diaria: f64,
    pub estoque_seguranca: i64,
    pub estoque_minimo: i64,
    pub estoque_total_recomendado: i64,
    pub qtd_sugerida: i64,
    pub volume_janela: i64,
    pub dias_com_vendas: i64,
    pub outliers_detectados: i64,
    pub coef_variacao: f64,
}

#[derive(Serialize)]
pub struct PontoDto {
    pub data: String,
    pub valor: i64,
}

impl From<PontoSerie> for PontoDto {
    fn from(p: PontoSerie) -> Self {
        Self {
            data: p.data.to_string(),
            valor: p.valor,
        }
    }
}

#[derive(Serialize)]
pub struct DetalheProdutoDto {
    pub codigo_estoque: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub classe: String,
    pub status: String,
    pub fora_de_linha: bool,
    pub percentual_acumulado: Option<f64>,
    pub dt_ref: String,
    pub regra: RegraClasseDto,
    pub metricas: MetricasDto,
    pub vendas_90d: Vec<PontoDto>,
    pub estoque_90d: Vec<PontoDto>,
}

/// Detalhe completo de um produto (autenticado — qualquer papel lê).
///
/// # Errors
/// [`ApiError::NaoEncontrado`] se o produto não existir; [`ApiError::Interno`] em falha de leitura.
pub async fn produto(
    State(estado): State<AppState>,
    Path(codigo): Path<String>,
) -> Result<Json<DetalheProdutoDto>, ApiError> {
    let d = detalhe::produto(&estado.pool, &codigo)
        .await?
        .ok_or(ApiError::NaoEncontrado)?;
    let vendas = detalhe::vendas_90d(&estado.pool, &codigo, d.dt_ref).await?;
    let estoque = detalhe::estoque_90d(&estado.pool, &codigo, d.dt_ref).await?;

    let regra = RegraClasseDto {
        meta_cobertura_dias: meta_cobertura(&estado.config, &d.classe),
        limiar_critico_dias: limiar_critico(&estado.config, &d.classe),
        fator_estoque: d.fator_estoque,
        justificativa: justificativa(&d),
    };
    let metricas = MetricasDto {
        qtd_estoque: d.qtd_estoque,
        qtd_reserva: d.qtd_reserva,
        qtd_disponivel: d.qtd_disponivel,
        cobertura_dias: d.cobertura_dias,
        media_diaria: d.media_diaria,
        estoque_seguranca: d.estoque_seguranca,
        estoque_minimo: d.estoque_minimo,
        estoque_total_recomendado: d.estoque_total_recomendado,
        qtd_sugerida: d.qtd_sugerida,
        volume_janela: d.volume_janela,
        dias_com_vendas: d.dias_com_vendas,
        outliers_detectados: d.outliers_detectados,
        coef_variacao: d.coef_variacao,
    };

    Ok(Json(DetalheProdutoDto {
        codigo_estoque: d.codigo_estoque,
        sku: d.sku,
        produto: d.produto,
        configuracao: d.configuracao,
        classe: d.classe,
        status: d.status,
        fora_de_linha: d.fora_de_linha,
        percentual_acumulado: d.percentual_acumulado,
        dt_ref: d.dt_ref.to_string(),
        regra,
        metricas,
        vendas_90d: vendas.into_iter().map(Into::into).collect(),
        estoque_90d: estoque.into_iter().map(Into::into).collect(),
    }))
}

/// Meta de cobertura (dias) da classe, vinda da config (doc 02 §3.6 / §11).
fn meta_cobertura(c: &Config, classe: &str) -> u32 {
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

/// Limiar de criticidade (dias) da classe — só A/B/C têm limiar próprio (doc 02 §5.2 / §11).
fn limiar_critico(c: &Config, classe: &str) -> Option<u32> {
    let l = &c.limiar_critico_dias;
    match classe {
        "A" => Some(l.a),
        "B" => Some(l.b),
        "C" => Some(l.c),
        _ => None,
    }
}

/// Justificativa textual da classe a partir dos fatos já calculados (não recalcula nada).
fn justificativa(d: &DetalheProduto) -> String {
    match d.classe.as_str() {
        "A" | "B" | "C" => {
            let pareto = d
                .percentual_acumulado
                .map_or_else(|| "—".to_owned(), |p| format!("{p:.1}%"));
            format!("Classificação por Pareto na janela de 18 meses (volume acumulado {pareto}).")
        }
        "D" => "Sem vendas relevantes na janela — classe de baixa prioridade.".to_owned(),
        "F" => "Produto fora de linha — meta mínima de cobertura.".to_owned(),
        "N" => "Produto novo — histórico insuficiente; meta provisória.".to_owned(),
        _ => "Classe não reconhecida.".to_owned(),
    }
}
