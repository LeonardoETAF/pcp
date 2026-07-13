//! Solicitação de Produção (doc 03 §4.3) — escrita do usuário, auditada (CLAUDE.md §7.2/§7.5).
//! Criar: qualquer autenticado (analista+). Transicionar (aprovar/avançar/recusar): gestor+.
//! A quantidade/prioridade default vêm da recomendação (doc 02 §7); a máquina de estados é do
//! `pcp-core` (regra única, §3.1).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::{Extension, Json};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use pcp_config::Config;
use pcp_core::solicitacao::estado::transicionar;
use pcp_core::{aprovacao_automatica, EstadoSolicitacao, PrioridadeProducao};
use pcp_db::solicitacoes::{self, NovaSolicitacao, Solicitacao};

use crate::estado::AppState;
use crate::recomendacao;
use sf_auth::Claims;
use sf_auth::Papel;
use sf_http::ApiError;

#[derive(Serialize)]
pub struct SolicitacaoDto {
    pub id: String,
    pub codigo_estoque: String,
    pub qtd_solicitada: i64,
    pub prioridade: String,
    pub lead_time_dias: i32,
    pub prazo: String,
    pub solicitante_id: String,
    pub justificativa: Option<String>,
    pub estado: String,
    pub criado_em: String,
    pub atualizado_em: String,
}

impl From<Solicitacao> for SolicitacaoDto {
    fn from(s: Solicitacao) -> Self {
        Self {
            id: s.id.to_string(),
            codigo_estoque: s.codigo_estoque,
            qtd_solicitada: s.qtd_solicitada,
            prioridade: s.prioridade,
            lead_time_dias: s.lead_time_dias,
            prazo: s.prazo.to_string(),
            solicitante_id: s.solicitante_id.to_string(),
            justificativa: s.justificativa,
            estado: s.estado,
            criado_em: iso(s.criado_em),
            atualizado_em: iso(s.atualizado_em),
        }
    }
}

#[derive(Deserialize)]
pub struct NovaSolicitacaoReq {
    pub codigo_estoque: String,
    pub qtd_solicitada: i64,
    pub prioridade: String,
    pub justificativa: Option<String>,
    /// Prazo (ISO `YYYY-MM-DD`); se ausente, usa hoje + lead time da prioridade.
    pub prazo: Option<String>,
}

#[derive(Deserialize)]
pub struct ParamsListar {
    pub codigo: String,
}

#[derive(Deserialize)]
pub struct TransicaoReq {
    pub para_estado: String,
    pub observacao: Option<String>,
}

/// `POST /pcp/solicitacoes` — cria uma solicitação (analista+). Aprovação automática (doc 02 §7.2)
/// cria já em `aprovada`, registrando o evento; caso contrário fica `pendente`.
///
/// # Errors
/// [`ApiError::Requisicao`] (dados inválidos); [`ApiError`] em falha de escrita.
pub async fn criar(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<NovaSolicitacaoReq>,
) -> Result<(StatusCode, Json<SolicitacaoDto>), ApiError> {
    if req.qtd_solicitada <= 0 {
        return Err(ApiError::Requisicao(
            "quantidade deve ser positiva".to_owned(),
        ));
    }
    let prioridade = recomendacao::prioridade_de(&req.prioridade)
        .ok_or_else(|| ApiError::Requisicao("prioridade inválida".to_owned()))?;
    let config = estado.config();
    let lead = lead_time(&config, prioridade);
    let prazo = prazo_ou_default(req.prazo.as_deref(), lead)?;

    let auto = aprovacao_automatica(
        req.qtd_solicitada,
        prioridade,
        i64::from(config.reposicao.aprovacao_automatica.qtd_max),
        recomendacao::excecao_aprovacao(&config),
    );
    let estado_inicial = if auto {
        EstadoSolicitacao::Aprovada
    } else {
        EstadoSolicitacao::Pendente
    };

    let nova = NovaSolicitacao {
        codigo_estoque: req.codigo_estoque.trim(),
        qtd_solicitada: req.qtd_solicitada,
        prioridade: recomendacao::prioridade_str(prioridade),
        lead_time_dias: lead,
        prazo,
        solicitante_id: usuario_id(&claims)?,
        justificativa: req.justificativa.as_deref(),
        estado_inicial: estado_inicial.codigo(),
    };
    let s = solicitacoes::criar(&estado.pool, &nova).await?;
    Ok((StatusCode::CREATED, Json(s.into())))
}

/// `GET /pcp/solicitacoes?codigo=...` — solicitações do produto (autenticado).
///
/// # Errors
/// [`ApiError`] em falha de leitura.
pub async fn listar(
    State(estado): State<AppState>,
    Query(params): Query<ParamsListar>,
) -> Result<Json<Vec<SolicitacaoDto>>, ApiError> {
    let itens = solicitacoes::listar_por_produto(&estado.pool, &params.codigo).await?;
    Ok(Json(itens.into_iter().map(Into::into).collect()))
}

/// `POST /pcp/solicitacoes/{id}/transicao` — aprova/avança/recusa (gestor+). Valida a máquina de
/// estados no `pcp-core` e registra o evento de auditoria (§7.5).
///
/// # Errors
/// [`ApiError::Proibido`] (não gestor); [`ApiError::Requisicao`] (transição inválida);
/// [`ApiError::NaoEncontrado`] (id inexistente).
pub async fn transicionar_estado(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
    Path(id): Path<Uuid>,
    Json(req): Json<TransicaoReq>,
) -> Result<Json<SolicitacaoDto>, ApiError> {
    claims.exige(Papel::Gestor)?;
    let para = EstadoSolicitacao::tentar_de(&req.para_estado)
        .ok_or_else(|| ApiError::Requisicao("estado de destino inválido".to_owned()))?;
    let atual = solicitacoes::estado_atual(&estado.pool, id)
        .await?
        .ok_or(ApiError::NaoEncontrado)?;
    let de = EstadoSolicitacao::tentar_de(&atual).ok_or(ApiError::Interno)?;
    transicionar(de, para).map_err(|e| ApiError::Requisicao(e.to_string()))?;

    let s = solicitacoes::transicionar(
        &estado.pool,
        id,
        de.codigo(),
        para.codigo(),
        usuario_id(&claims)?,
        req.observacao.as_deref(),
    )
    .await?;
    Ok(Json(s.into()))
}

/// Lead time (dias) da prioridade, vindo da config (doc 02 §7.2).
fn lead_time(c: &Config, p: PrioridadeProducao) -> i32 {
    let dias = match p {
        PrioridadeProducao::Alta => c.reposicao.lead_time_dias.alta,
        PrioridadeProducao::Media => c.reposicao.lead_time_dias.media,
        PrioridadeProducao::Baixa => c.reposicao.lead_time_dias.baixa,
    };
    i32::try_from(dias).unwrap_or(i32::MAX)
}

/// Prazo informado (ISO) ou, na ausência, hoje + lead time.
fn prazo_ou_default(prazo: Option<&str>, lead: i32) -> Result<NaiveDate, ApiError> {
    match prazo {
        Some(s) => NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map_err(|_| ApiError::Requisicao("prazo inválido (use AAAA-MM-DD)".to_owned())),
        None => Ok(Utc::now().date_naive() + Duration::days(i64::from(lead))),
    }
}

/// Id do usuário autenticado (sub = uuid).
fn usuario_id(claims: &Claims) -> Result<Uuid, ApiError> {
    Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Interno)
}

fn iso(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}
