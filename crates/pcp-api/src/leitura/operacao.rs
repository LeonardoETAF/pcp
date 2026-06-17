//! `GET /pcp/admin/pipeline` e `/pcp/admin/saude` — painel de operação (doc 05 §3/§4),
//! **admin-only** (§7.3). Status das execuções do pipeline e health checks. Os limiares do §4
//! são avaliados AQUI e a API entrega o veredito pronto (ok/atenção/crítico) — frontend burro
//! (§3). Somente leitura.

use axum::extract::State;
use axum::http::StatusCode;
use axum::{Extension, Json};
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use pcp_db::operacao::{self, MetricasSaude, RegistroExecucao};

use crate::erro::ApiError;
use crate::estado::AppState;
use crate::jwt::Claims;
use crate::papel::Papel;

/// Limite de segurança do intervalo de reprocesso (evita varreduras gigantes acidentais).
const MAX_DIAS_REPROCESSO: i64 = 92;

/// Quantas execuções recentes do pipeline expor no painel.
const LIMITE_EXECUCOES: i64 = 60;

#[derive(Serialize)]
pub struct ExecucaoDto {
    pub data_ref: String,
    pub modulo: String,
    pub status: String,
    pub linhas_afetadas: i64,
    pub duracao_ms: i64,
    pub erro: Option<String>,
    pub inicio: String,
    pub fim: String,
}

impl From<RegistroExecucao> for ExecucaoDto {
    fn from(r: RegistroExecucao) -> Self {
        Self {
            data_ref: r.data_ref.to_string(),
            modulo: r.modulo,
            status: r.status,
            linhas_afetadas: r.linhas_afetadas,
            duracao_ms: r.duracao_ms,
            erro: r.erro,
            inicio: r.inicio.to_rfc3339(),
            fim: r.fim.to_rfc3339(),
        }
    }
}

/// Execuções recentes do pipeline (admin).
///
/// # Errors
/// [`ApiError::Proibido`] se não for admin; [`ApiError::Interno`] em falha de leitura.
pub async fn pipeline(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<ExecucaoDto>>, ApiError> {
    claims.exige(Papel::Admin)?;
    let linhas = operacao::execucoes_recentes(&estado.pool, LIMITE_EXECUCOES).await?;
    Ok(Json(linhas.into_iter().map(Into::into).collect()))
}

/// Resultado de uma verificação de saúde (doc 05 §4): `status` ∈ ok | atencao | critico.
#[derive(Serialize)]
pub struct VerificacaoDto {
    pub nome: String,
    pub status: String,
    pub detalhe: String,
}

#[derive(Serialize)]
pub struct SaudeDto {
    pub gerado_em: String,
    pub verificacoes: Vec<VerificacaoDto>,
}

/// Health checks do pipeline/dados (admin) — doc 05 §4.
///
/// # Errors
/// [`ApiError::Proibido`] se não for admin; [`ApiError::Interno`] em falha de leitura.
pub async fn saude(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<SaudeDto>, ApiError> {
    claims.exige(Papel::Admin)?;
    let hoje = Local::now().date_naive();
    let metricas = operacao::metricas_saude(&estado.pool, hoje).await?;
    Ok(Json(SaudeDto {
        gerado_em: Local::now().to_rfc3339(),
        verificacoes: avaliar(&metricas, hoje),
    }))
}

/// Pedido de reprocesso de um intervalo de datas (admin).
#[derive(Deserialize)]
pub struct ReprocessarReq {
    pub inicio: NaiveDate,
    pub fim: NaiveDate,
}

/// Reprocessa o pipeline de um intervalo de datas, de forma idempotente (doc 05 §1.3). Admin-only.
/// Dispara em segundo plano (pode demorar) e responde 202; o resultado aparece no painel de
/// execuções e nos health checks (o motor notifica o canal SSE ao terminar cada dia).
///
/// # Errors
/// [`ApiError::Proibido`] se não for admin; [`ApiError::Requisicao`] se o intervalo for inválido.
pub async fn reprocessar(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(req): Json<ReprocessarReq>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    claims.exige(Papel::Admin)?;
    if req.fim < req.inicio {
        return Err(ApiError::Requisicao(
            "data final antes da inicial".to_owned(),
        ));
    }
    if (req.fim - req.inicio).num_days() > MAX_DIAS_REPROCESSO {
        return Err(ApiError::Requisicao(format!(
            "intervalo excede o máximo de {MAX_DIAS_REPROCESSO} dias"
        )));
    }
    // Segundo plano: o reprocesso pode levar minutos; a transação de cada dia é idempotente.
    let pool = estado.pool.clone();
    let config = estado.config();
    let (inicio, fim) = (req.inicio, req.fim);
    tokio::spawn(async move {
        match pcp_engine::reprocessar_intervalo(&pool, &config, inicio, fim).await {
            Ok(r) => tracing::info!(dias = r.len(), %inicio, %fim, "reprocesso concluído"),
            Err(e) => tracing::error!(erro = %e, %inicio, %fim, "reprocesso falhou"),
        }
    });
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "mensagem": format!("Reprocesso de {inicio} a {fim} iniciado.") })),
    ))
}

fn ver(nome: &str, status: &str, detalhe: String) -> VerificacaoDto {
    VerificacaoDto {
        nome: nome.to_owned(),
        status: status.to_owned(),
        detalhe,
    }
}

/// Aplica os limiares do doc 05 §4 sobre as métricas cruas. Limiares são spec de monitoramento
/// (doc 05 §4), não constantes de negócio do §11 — por isso vivem aqui, citando o §.
#[allow(clippy::cast_precision_loss)] // contagens/durações: conversão para f64 só para razão/%
fn avaliar(m: &MetricasSaude, hoje: NaiveDate) -> Vec<VerificacaoDto> {
    let mut v = Vec::new();

    // 1. Snapshot do dia presente (SLA até 05:00).
    v.push(match m.data_ref_snapshot {
        Some(d) if d == hoje => ver("Snapshot do dia", "ok", format!("Presente ({d}).")),
        Some(d) => ver(
            "Snapshot do dia",
            "critico",
            format!("Desatualizado — último em {d}."),
        ),
        None => ver(
            "Snapshot do dia",
            "critico",
            "Nenhum snapshot de estoque.".to_owned(),
        ),
    });

    // 2. Variação do nº de produtos vs dia anterior (> ±10% → investigar).
    if m.produtos_anterior > 0 {
        let pct =
            (m.produtos_recente - m.produtos_anterior) as f64 / m.produtos_anterior as f64 * 100.0;
        let st = if pct.abs() > 10.0 { "atencao" } else { "ok" };
        v.push(ver(
            "Variação de produtos",
            st,
            format!(
                "{} produtos ({pct:+.1}% vs dia anterior).",
                m.produtos_recente
            ),
        ));
    } else {
        v.push(ver(
            "Variação de produtos",
            "ok",
            format!(
                "{} produtos (sem dia anterior para comparar).",
                m.produtos_recente
            ),
        ));
    }

    // 3. Última execução do pipeline sem erro de módulo.
    v.push(if m.ultima_execucao_com_erro {
        ver(
            "Última execução",
            "critico",
            "Há módulo com erro na última execução.".to_owned(),
        )
    } else {
        ver("Última execução", "ok", "Sem erros de módulo.".to_owned())
    });

    // 4. Duração do pipeline (> 5× média → investigar).
    if m.duracao_media_ms > 0.0 {
        let razao = m.duracao_ultima_ms as f64 / m.duracao_media_ms;
        let st = if razao > 5.0 { "atencao" } else { "ok" };
        v.push(ver(
            "Duração do pipeline",
            st,
            format!(
                "Última {} ms vs média {:.0} ms ({razao:.1}×).",
                m.duracao_ultima_ms, m.duracao_media_ms
            ),
        ));
    }

    // 5. Zero alertas por > 7 dias (lógica pode estar quebrada).
    v.push(match m.dias_sem_alerta {
        Some(d) if d > 7 => ver(
            "Geração de alertas",
            "atencao",
            format!("Sem alertas há {d} dias — verificar a lógica."),
        ),
        Some(d) => ver(
            "Geração de alertas",
            "ok",
            format!("Último alerta há {d} dia(s)."),
        ),
        None => ver(
            "Geração de alertas",
            "atencao",
            "Nunca houve alertas.".to_owned(),
        ),
    });

    // 6. CV médio do catálogo (> 0,5 → dados suspeitos).
    if let Some(cv) = m.cv_medio {
        let st = if cv > 0.5 { "atencao" } else { "ok" };
        v.push(ver("CV médio do catálogo", st, format!("{cv:.2}.")));
    }

    v
}
