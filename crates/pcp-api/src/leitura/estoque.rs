//! `GET /pcp/estoque` — tabela de produtos ativos paginada no servidor (doc 03 §3 / doc 04 §6.2).
//! Filtros: classe, status, busca textual e ordenação (allowlist). Só lê `produto_ativo` (§3/§15).

use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use pcp_db::leituras::{self, FiltroEstoque, LinhaEstoque};

use crate::erro::ApiError;
use crate::estado::AppState;

/// Tamanho de página padrão e teto (doc 03 §3.4: até 1000 por página; paginação no servidor).
const LIMITE_PADRAO: i64 = 50;
const LIMITE_MAX: i64 = 1000;

#[derive(Deserialize)]
pub struct ParamsEstoque {
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
    pub qtd_estoque: i64,
    pub qtd_reserva: i64,
    pub qtd_disponivel: i64,
    pub media_diaria: f64,
    pub cobertura_dias: f64,
    pub estoque_minimo: i64,
    pub estoque_total_recomendado: i64,
    pub volume_janela: i64,
    pub status: String,
    pub qtd_sugerida: i64,
    pub fora_de_linha: bool,
    /// `em_producao` | `aguardando` | `recem_produzido` | ausente.
    pub estado_producao: Option<String>,
}

impl From<LinhaEstoque> for LinhaEstoqueDto {
    fn from(l: LinhaEstoque) -> Self {
        Self {
            codigo_estoque: l.codigo_estoque,
            sku: l.sku,
            produto: l.produto,
            configuracao: l.configuracao,
            classe: l.classe,
            qtd_estoque: l.qtd_estoque,
            qtd_reserva: l.qtd_reserva,
            qtd_disponivel: l.qtd_disponivel,
            media_diaria: l.media_diaria,
            cobertura_dias: l.cobertura_dias,
            estoque_minimo: l.estoque_minimo,
            estoque_total_recomendado: l.estoque_total_recomendado,
            volume_janela: l.volume_janela,
            status: l.status,
            qtd_sugerida: l.qtd_sugerida,
            fora_de_linha: l.fora_de_linha,
            estado_producao: l.estado_producao,
        }
    }
}

/// Quantos itens cada classe traria com o filtro atual (busca/status), ignorando a classe
/// escolhida — é o número que cada botão de classe exibe.
#[derive(Serialize)]
pub struct ContagemClasseDto {
    pub classe: String,
    pub quantidade: i64,
}

#[derive(Serialize)]
pub struct PaginaEstoqueDto {
    pub itens: Vec<LinhaEstoqueDto>,
    pub total: i64,
    pub limite: i64,
    pub deslocamento: i64,
    /// Vem junto com a página: um round-trip só, e nunca dessincroniza da lista.
    pub contagem_classes: Vec<ContagemClasseDto>,
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
    let filtro = FiltroEstoque {
        classe: params.classe.as_deref(),
        status: params.status.as_deref(),
        busca: params.busca.as_deref().filter(|s| !s.is_empty()),
        ordem: params.ordem.as_deref().unwrap_or("sugerida_desc"),
        cobertura_min: params.cobertura_min,
        cobertura_max: params.cobertura_max,
        apenas_sugestao: params.apenas_sugestao,
        apenas_fora_linha: params.apenas_fora_linha,
    };
    let contagens = leituras::contagem_classes(&estado.pool, &filtro).await?;
    let recem = i32::try_from(estado.config().producao.recem_produzido_dias).unwrap_or(7);
    let pagina =
        leituras::produtos_paginado(&estado.pool, filtro, recem, limite, deslocamento).await?;
    Ok(Json(PaginaEstoqueDto {
        itens: pagina.itens.into_iter().map(Into::into).collect(),
        total: pagina.total,
        limite,
        deslocamento,
        contagem_classes: contagens
            .into_iter()
            .map(|c| ContagemClasseDto {
                classe: c.classe,
                quantidade: c.quantidade,
            })
            .collect(),
    }))
}
