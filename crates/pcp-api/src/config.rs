//! Configuração de negócio (doc 02 §11 / doc 03 §8). Leitura por qualquer autenticado; edição
//! só pelo GESTOR (§7.3), validada (`pcp-config`), persistida e **recarregada a quente** com
//! auditoria por constante alterada (§7.5). Frontend burro: envia o documento completo.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use pcp_db::config_persist::{self, MudancaConfig};

use crate::erro::ApiError;
use crate::estado::AppState;
use crate::jwt::Claims;
use crate::papel::Papel;

/// `GET /pcp/config` — configuração de negócio vigente (autenticado).
#[allow(clippy::unused_async)] // handler assíncrono exigido pelo Axum
pub async fn obter(State(estado): State<AppState>) -> Json<pcp_config::Config> {
    Json((*estado.config()).clone())
}

/// `PUT /pcp/config` — substitui a configuração (gestor+). Valida, persiste, audita e recarrega.
///
/// # Errors
/// [`ApiError::Proibido`] (não gestor); [`ApiError::Requisicao`] (config inválida); [`ApiError`]
/// em falha de escrita.
pub async fn salvar(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(corpo): Json<Value>,
) -> Result<Json<pcp_config::Config>, ApiError> {
    claims.exige(Papel::Gestor)?;
    let por_id = Uuid::parse_str(&claims.sub).map_err(|_| ApiError::Interno)?;

    let nova: pcp_config::Config = serde_json::from_value(corpo)
        .map_err(|e| ApiError::Requisicao(format!("configuração malformada: {e}")))?;
    pcp_config::validar(&nova).map_err(|e| ApiError::Requisicao(e.to_string()))?;

    let atual = estado.config();
    let mudancas = diferencas(&atual, &nova);
    if mudancas.is_empty() {
        return Ok(Json((*atual).clone()));
    }
    let valor = serde_json::to_value(&nova).map_err(|_| ApiError::Interno)?;
    config_persist::salvar(&estado.pool, &valor, por_id, &mudancas).await?;
    estado.trocar_config(Arc::new(nova.clone()));
    Ok(Json(nova))
}

#[derive(Deserialize)]
pub struct ParamsAuditoria {
    pub limite: Option<i64>,
}

#[derive(Serialize)]
pub struct EntradaAuditoriaDto {
    pub chave: String,
    pub valor_anterior: Option<String>,
    pub valor_novo: Option<String>,
    pub por_id: String,
    pub em: String,
}

/// `GET /pcp/config/auditoria` — trilha de mudanças de configuração (gestor+).
///
/// # Errors
/// [`ApiError::Proibido`] (não gestor); [`ApiError`] em falha de leitura.
pub async fn auditoria(
    State(estado): State<AppState>,
    Extension(claims): Extension<Claims>,
    Query(params): Query<ParamsAuditoria>,
) -> Result<Json<Vec<EntradaAuditoriaDto>>, ApiError> {
    claims.exige(Papel::Gestor)?;
    let limite = params.limite.unwrap_or(50).clamp(1, 500);
    let itens = config_persist::auditoria(&estado.pool, limite).await?;
    Ok(Json(
        itens
            .into_iter()
            .map(|e| EntradaAuditoriaDto {
                chave: e.chave,
                valor_anterior: e.valor_anterior,
                valor_novo: e.valor_novo,
                por_id: e.por_id.to_string(),
                em: e.em.to_rfc3339(),
            })
            .collect(),
    ))
}

/// Constantes alteradas entre duas configs (folhas do JSON com valor diferente).
fn diferencas(atual: &pcp_config::Config, nova: &pcp_config::Config) -> Vec<MudancaConfig> {
    let (mut a, mut b) = (BTreeMap::new(), BTreeMap::new());
    if let Ok(v) = serde_json::to_value(atual) {
        achatar("", &v, &mut a);
    }
    if let Ok(v) = serde_json::to_value(nova) {
        achatar("", &v, &mut b);
    }
    b.into_iter()
        .filter(|(chave, novo)| a.get(chave) != Some(novo))
        .map(|(chave, novo)| MudancaConfig {
            valor_anterior: a.get(&chave).cloned(),
            valor_novo: Some(novo),
            chave,
        })
        .collect()
}

/// Achata um JSON em pares `caminho → valor` (folhas), ex.: `classificacao.pareto_a → 80`.
fn achatar(prefixo: &str, v: &Value, out: &mut BTreeMap<String, String>) {
    if let Value::Object(mapa) = v {
        for (k, val) in mapa {
            let caminho = if prefixo.is_empty() {
                k.clone()
            } else {
                format!("{prefixo}.{k}")
            };
            achatar(&caminho, val, out);
        }
    } else {
        out.insert(prefixo.to_owned(), v.to_string());
    }
}
