//! Ponte com a `pcp-api` via *server functions* do Leptos: o corpo roda no servidor SSR
//! (server-to-server), evitando CORS e mantendo segredos fora do WASM. Frontend burro: só
//! repassa credenciais e devolve o token; nenhuma regra de negócio aqui (CLAUDE.md §3/§7).

use leptos::prelude::*;
use serde::{Deserialize, Serialize};

/// Contagem `(rótulo, quantidade)` das distribuições do painel.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Contagem {
    pub rotulo: String,
    pub quantidade: i64,
}

/// Métricas agregadas do painel (`GET /pcp/dashboard`). Valores já calculados pelo motor.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PainelResumo {
    pub data_ref: Option<String>,
    pub total_produtos: i64,
    pub total_sugerido: i64,
    pub cobertura_media: Option<f64>,
    pub por_classe: Vec<Contagem>,
    pub por_status: Vec<Contagem>,
}

/// Linha de produto da tabela de estoque (`GET /pcp/estoque`). Espelha o DTO da `pcp-api` —
/// todos os valores já calculados pelo motor; o frontend só exibe (CLAUDE.md §3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinhaEstoque {
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
}

/// Página de produtos (ignora `limite`/`deslocamento` do payload — só precisamos de itens/total).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PaginaEstoque {
    pub itens: Vec<LinhaEstoque>,
    pub total: i64,
}

/// Parâmetros da consulta de estoque (filtros + ordenação + paginação no servidor — doc 03 §3.2).
/// Um único conceito de consulta, reutilizado pela tabela e pelo dashboard (CLAUDE.md §13).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConsultaEstoque {
    pub classe: Option<String>,
    pub status: Option<String>,
    pub busca: Option<String>,
    pub ordem: Option<String>,
    pub cobertura_min: Option<f64>,
    pub cobertura_max: Option<f64>,
    pub apenas_sugestao: bool,
    pub apenas_fora_linha: bool,
    pub limite: i64,
    pub deslocamento: i64,
}

/// Métricas do painel (`GET /pcp/dashboard`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = Painel, prefix = "/api")]
pub async fn painel(token: String) -> Result<PainelResumo, ServerFnError> {
    obter_json("/pcp/dashboard", &token).await
}

/// Resumo por classe do dashboard executivo (`GET /pcp/dashboard/classes`): metas físicas (§9.1)
/// e cobertura média por classe. Valores já calculados/comparados pela API (frontend burro).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ClasseResumo {
    pub classe: String,
    pub qtd_produtos: i64,
    pub estoque_fisico: i64,
    pub pct_fisico_real: f64,
    pub pct_fisico_meta: Option<u32>,
    pub meta_atingida: Option<bool>,
    pub cobertura_media: Option<f64>,
}

/// Resumo por classe (metas físicas + cobertura) — `GET /pcp/dashboard/classes`.
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = DashboardClasses, prefix = "/api")]
pub async fn dashboard_classes(token: String) -> Result<Vec<ClasseResumo>, ServerFnError> {
    obter_json("/pcp/dashboard/classes", &token).await
}

/// Produtos ativos paginados (`GET /pcp/estoque`) com filtros, busca, ordenação e paginação.
/// `reqwest::query` cuida do *url-encoding* (busca livre do usuário pode ter espaços/acentos).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = ListarEstoque, prefix = "/api")]
pub async fn estoque(
    token: String,
    consulta: ConsultaEstoque,
) -> Result<PaginaEstoque, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let mut params = parametros_consulta(&consulta);
    params.push(("limite", consulta.limite.to_string()));
    params.push(("deslocamento", consulta.deslocamento.to_string()));
    let resposta = reqwest::Client::new()
        .get(format!("{base}/pcp/estoque"))
        .query(&params)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao carregar dados"));
    }
    resposta
        .json::<PaginaEstoque>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Exporta o filtro completo de estoque (`GET /pcp/estoque/exportar`) em CSV ou JSON. Devolve o
/// conteúdo como texto; o download em si é disparado no cliente (CLAUDE.md §12). Sem paginação.
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = ExportarEstoque, prefix = "/api")]
pub async fn exportar_estoque(
    token: String,
    consulta: ConsultaEstoque,
    formato: String,
) -> Result<String, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let mut params = parametros_consulta(&consulta);
    params.push(("formato", formato));
    let resposta = reqwest::Client::new()
        .get(format!("{base}/pcp/estoque/exportar"))
        .query(&params)
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao exportar dados"));
    }
    resposta
        .text()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Filtro salvo do usuário (`/pcp/estoque/filtros`). `filtro` é o JSON opaco de [`ConsultaEstoque`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FiltroSalvo {
    pub id: String,
    pub nome: String,
    pub filtro: serde_json::Value,
}

/// Lista os filtros salvos do usuário (`GET /pcp/estoque/filtros`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = ListarFiltros, prefix = "/api")]
pub async fn listar_filtros(token: String) -> Result<Vec<FiltroSalvo>, ServerFnError> {
    obter_json("/pcp/estoque/filtros", &token).await
}

/// Salva (ou atualiza por nome) um filtro do usuário (`POST /pcp/estoque/filtros`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou nome inválido.
#[server(name = SalvarFiltro, prefix = "/api")]
pub async fn salvar_filtro(
    token: String,
    nome: String,
    filtro: serde_json::Value,
) -> Result<FiltroSalvo, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .post(format!("{base}/pcp/estoque/filtros"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "nome": nome, "filtro": filtro }))
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao salvar o filtro"));
    }
    resposta
        .json::<FiltroSalvo>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Exclui um filtro salvo do usuário (`DELETE /pcp/estoque/filtros/{id}`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede ou sessão expirada.
#[server(name = ExcluirFiltro, prefix = "/api")]
pub async fn excluir_filtro(token: String, id: String) -> Result<(), ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .delete(format!("{base}/pcp/estoque/filtros/{id}"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao excluir o filtro"));
    }
    Ok(())
}

/// Um ponto de série (dia ISO → valor) dos gráficos de 90 dias.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Ponto {
    pub data: String,
    pub valor: i64,
}

/// Regra da classe aplicada ao produto (valores vindos da config — o front só exibe).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RegraClasse {
    pub meta_cobertura_dias: u32,
    pub limiar_critico_dias: Option<u32>,
    pub fator_estoque: f64,
    pub justificativa: String,
}

/// Métricas do produto (já calculadas pelo motor).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MetricasProduto {
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

/// Detalhe completo de um produto (`GET /pcp/produto/{codigo}`). Frontend burro: só exibe (§3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DetalheProduto {
    pub codigo_estoque: String,
    pub sku: Option<String>,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub classe: String,
    pub status: String,
    pub fora_de_linha: bool,
    pub percentual_acumulado: Option<f64>,
    pub dt_ref: String,
    pub regra: RegraClasse,
    pub metricas: MetricasProduto,
    pub recomendacao: Recomendacao,
    pub vendas_90d: Vec<Ponto>,
    pub estoque_90d: Vec<Ponto>,
}

/// Recomendação para gerar a solicitação de produção (doc 02 §7.2) — default editável.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Recomendacao {
    pub qtd_sugerida: i64,
    pub prioridade: String,
    pub lead_time_dias: i64,
    pub prazo_sugerido: String,
    pub aprovacao_automatica: bool,
}

/// Solicitação de produção persistida (`/pcp/solicitacoes`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Solicitacao {
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

/// Carrega o detalhe de um produto. `Ok(None)` se não existir (404) — o resto é erro.
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = DetalheProdutoFn, prefix = "/api")]
pub async fn produto_detalhe(
    token: String,
    codigo: String,
) -> Result<Option<DetalheProduto>, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .get(format!("{base}/pcp/produto/{codigo}"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if resposta.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao carregar o produto"));
    }
    resposta
        .json::<DetalheProduto>()
        .await
        .map(Some)
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Lista as solicitações de produção de um produto (`GET /pcp/solicitacoes?codigo=`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = ListarSolicitacoes, prefix = "/api")]
pub async fn listar_solicitacoes(
    token: String,
    codigo: String,
) -> Result<Vec<Solicitacao>, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .get(format!("{base}/pcp/solicitacoes"))
        .query(&[("codigo", codigo)])
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao carregar solicitações"));
    }
    resposta
        .json::<Vec<Solicitacao>>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Cria uma solicitação de produção (`POST /pcp/solicitacoes`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou dados inválidos.
#[server(name = CriarSolicitacao, prefix = "/api")]
pub async fn criar_solicitacao(
    token: String,
    codigo_estoque: String,
    qtd_solicitada: i64,
    prioridade: String,
    justificativa: String,
) -> Result<Solicitacao, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let justificativa = (!justificativa.trim().is_empty()).then_some(justificativa);
    let resposta = reqwest::Client::new()
        .post(format!("{base}/pcp/solicitacoes"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "codigo_estoque": codigo_estoque,
            "qtd_solicitada": qtd_solicitada,
            "prioridade": prioridade,
            "justificativa": justificativa,
        }))
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao criar a solicitação"));
    }
    resposta
        .json::<Solicitacao>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Transiciona o estado de uma solicitação (`POST /pcp/solicitacoes/{id}/transicao`) — gestor+.
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sem permissão ou transição inválida.
#[server(name = TransicionarSolicitacao, prefix = "/api")]
pub async fn transicionar_solicitacao(
    token: String,
    id: String,
    para_estado: String,
) -> Result<Solicitacao, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .post(format!("{base}/pcp/solicitacoes/{id}/transicao"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "para_estado": para_estado }))
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if resposta.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(ServerFnError::new("apenas gestor pode alterar o estado"));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao atualizar a solicitação"));
    }
    resposta
        .json::<Solicitacao>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Papel do usuário autenticado (`GET /pcp/me`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede ou sessão inválida.
#[server(name = Perfil, prefix = "/api")]
pub async fn perfil(token: String) -> Result<String, ServerFnError> {
    #[derive(Deserialize)]
    struct Me {
        papel: String,
    }
    let me: Me = obter_json("/pcp/me", &token).await?;
    Ok(me.papel)
}

/// Monta os parâmetros comuns de consulta de estoque (filtros + faixas + switches) para a query
/// string. Compartilhado entre listagem e exportação — um só lugar (CLAUDE.md §13).
#[cfg(feature = "ssr")]
fn parametros_consulta(c: &ConsultaEstoque) -> Vec<(&'static str, String)> {
    let mut params: Vec<(&'static str, String)> = Vec::new();
    for (chave, valor) in [
        ("classe", c.classe.clone()),
        ("status", c.status.clone()),
        ("busca", c.busca.clone()),
        ("ordem", c.ordem.clone()),
    ] {
        if let Some(v) = valor.filter(|s| !s.is_empty()) {
            params.push((chave, v));
        }
    }
    if let Some(v) = c.cobertura_min {
        params.push(("cobertura_min", v.to_string()));
    }
    if let Some(v) = c.cobertura_max {
        params.push(("cobertura_max", v.to_string()));
    }
    if c.apenas_sugestao {
        params.push(("apenas_sugestao", "true".to_owned()));
    }
    if c.apenas_fora_linha {
        params.push(("apenas_fora_linha", "true".to_owned()));
    }
    params
}

/// Helper (só servidor): GET autenticado na `pcp-api` e desserialização do JSON.
#[cfg(feature = "ssr")]
async fn obter_json<T: serde::de::DeserializeOwned>(
    caminho: &str,
    token: &str,
) -> Result<T, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .get(format!("{base}{caminho}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao carregar dados"));
    }
    resposta
        .json::<T>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Alerta de produção como entregue pela API (`GET /pcp/alertas`). Valores já calculados pelo
/// motor — o frontend só exibe (CLAUDE.md §3). Espelha o DTO da `pcp-api`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertaResumo {
    pub codigo_estoque: String,
    pub prioridade: String,
    pub classe: String,
    pub qtd_sugerida: i64,
    pub cobertura_dias: f64,
    pub produto: Option<String>,
    pub configuracao: Option<String>,
    pub status: Option<String>,
}

/// Lista os alertas do dia (`GET /pcp/alertas`) com o token do usuário (Bearer).
///
/// # Errors
/// [`ServerFnError`] se a API não responder, a sessão expirar ou o corpo for inválido.
#[server(name = ListarAlertas, prefix = "/api")]
pub async fn alertas(token: String) -> Result<Vec<AlertaResumo>, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .get(format!("{base}/pcp/alertas"))
        .bearer_auth(&token)
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("sessão expirada — entre novamente"));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("falha ao carregar os alertas"));
    }
    resposta
        .json::<Vec<AlertaResumo>>()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

/// Autentica na `pcp-api` (`POST /auth/login`) e devolve o `access_token`.
///
/// # Errors
/// [`ServerFnError`] se a API não responder ou as credenciais forem inválidas.
#[server(name = Login, prefix = "/api")]
pub async fn login(email: String, senha: String) -> Result<String, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .post(format!("{base}/auth/login"))
        .json(&serde_json::json!({ "email": email, "senha": senha }))
        .send()
        .await
        .map_err(|e| ServerFnError::new(format!("falha ao contatar a API: {e}")))?;
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("credenciais inválidas"));
    }
    let corpo: serde_json::Value = resposta
        .json()
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;
    corpo["access_token"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| ServerFnError::new("resposta da API sem access_token"))
}
