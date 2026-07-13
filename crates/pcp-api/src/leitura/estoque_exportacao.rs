//! `GET /pcp/estoque/exportar` — exporta o **filtro completo** (não só a página) em CSV ou JSON
//! (doc 03 §3 / CLAUDE.md §12). CSV é UTF-8 **com BOM** e separador `;` (Excel BR), decimais com
//! vírgula. Reusa a mesma consulta filtrada da tabela (sem paginação) — nada de SQL duplicado.

use axum::extract::{Query, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use pcp_db::leituras::{self, FiltroEstoque, LinhaEstoque};

use super::estoque::LinhaEstoqueDto;
use crate::erro::ApiError;
use crate::estado::AppState;

#[derive(Deserialize)]
pub struct ParamsExport {
    pub classe: Option<String>,
    pub status: Option<String>,
    pub busca: Option<String>,
    pub ordem: Option<String>,
    pub cobertura_min: Option<f64>,
    pub cobertura_max: Option<f64>,
    #[serde(default)]
    pub apenas_sugestao: bool,
    #[serde(default)]
    pub apenas_fora_linha: bool,
    pub formato: Option<String>,
    /// Filtro por estado de produção (o mesmo da lista) — a exportação leva o filtro completo.
    pub producao: Option<String>,
}

/// Exporta os produtos do filtro atual (autenticado — qualquer papel lê).
///
/// # Errors
/// [`ApiError::Interno`] em falha de leitura ou serialização.
pub async fn exportar(
    State(estado): State<AppState>,
    Query(params): Query<ParamsExport>,
) -> Result<Response, ApiError> {
    let filtro = FiltroEstoque {
        classe: params.classe.as_deref(),
        status: params.status.as_deref(),
        busca: params.busca.as_deref().filter(|s| !s.is_empty()),
        ordem: params.ordem.as_deref().unwrap_or("sugerida_desc"),
        cobertura_min: params.cobertura_min,
        cobertura_max: params.cobertura_max,
        apenas_sugestao: params.apenas_sugestao,
        apenas_fora_linha: params.apenas_fora_linha,
        estado_producao: params.producao.as_deref().filter(|s| !s.is_empty()),
    };
    // Sem paginação: o filtro inteiro. i64::MAX como LIMIT devolve todas as linhas.
    let recem = i32::try_from(estado.config().producao.recem_produzido_dias).unwrap_or(2);
    let linhas = leituras::produtos_paginado(&estado.pool, filtro, recem, i64::MAX, 0)
        .await?
        .itens;

    if params.formato.as_deref() == Some("json") {
        let dtos: Vec<LinhaEstoqueDto> = linhas.into_iter().map(Into::into).collect();
        let corpo = serde_json::to_string_pretty(&dtos).map_err(|e| {
            tracing::error!(%e, "falha ao serializar exportação JSON");
            ApiError::Interno
        })?;
        Ok((
            [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
            corpo,
        )
            .into_response())
    } else {
        Ok((
            [(header::CONTENT_TYPE, "text/csv; charset=utf-8")],
            montar_csv(&linhas),
        )
            .into_response())
    }
}

/// Monta o CSV (UTF-8 com BOM, separador `;`, decimais com vírgula — Excel BR).
fn montar_csv(linhas: &[LinhaEstoque]) -> String {
    const CABECALHO: &str = "Código;SKU;Produto;Configuração;Classe;Estoque;Reserva;Disponível;\
        Média diária;Cobertura (dias);Estoque mínimo;Recomendada;Volume (janela);Status;\
        Produzir;Fora de Linha";
    let mut saida = String::from('\u{FEFF}'); // BOM para o Excel reconhecer UTF-8
    saida.push_str(CABECALHO);
    saida.push_str("\r\n");
    for l in linhas {
        let campos = [
            campo(&l.codigo_estoque),
            campo(l.sku.as_deref().unwrap_or_default()),
            campo(l.produto.as_deref().unwrap_or_default()),
            campo(l.configuracao.as_deref().unwrap_or_default()),
            campo(&l.classe),
            l.qtd_estoque.to_string(),
            l.qtd_reserva.to_string(),
            l.qtd_disponivel.to_string(),
            decimal(l.media_diaria),
            cobertura(l.cobertura_dias),
            l.estoque_minimo.to_string(),
            l.estoque_total_recomendado.to_string(),
            l.volume_janela.to_string(),
            campo(&l.status),
            l.qtd_sugerida.to_string(),
            if l.fora_de_linha { "Sim" } else { "Não" }.to_owned(),
        ];
        saida.push_str(&campos.join(";"));
        saida.push_str("\r\n");
    }
    saida
}

/// Escapa um campo textual de CSV: aspas se contiver `;`, `"` ou quebra de linha.
fn campo(valor: &str) -> String {
    if valor.contains([';', '"', '\n', '\r']) {
        format!("\"{}\"", valor.replace('"', "\"\""))
    } else {
        valor.to_owned()
    }
}

/// Decimal com vírgula (Excel BR): `50.0` → `50,0`.
fn decimal(valor: f64) -> String {
    format!("{valor:.1}").replace('.', ",")
}

/// Cobertura: sentinela 999 vira "Sem histórico" (§12); senão decimal com vírgula.
fn cobertura(valor: f64) -> String {
    if valor >= 999.0 {
        "Sem histórico".to_owned()
    } else {
        decimal(valor)
    }
}
