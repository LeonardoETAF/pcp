//! Ponte com a `pcp-api` via *server functions* do Leptos: o corpo roda no servidor SSR
//! (server-to-server), evitando CORS e mantendo segredos fora do WASM. Frontend burro: só
//! repassa credenciais e devolve o token; nenhuma regra de negócio aqui (CLAUDE.md §3/§7).

use leptos::prelude::*;
// Só o build SSR usa o derive nas structs locais das server fns (ex.: `perfil`); no WASM seria ocioso.
/// Erro de REDE ao chamar a `pcp-api`. O detalhe (URL, causa do `reqwest`) vai para o log; ao
/// usuário só a frase curta — ele não pode agir sobre um `ConnectionRefused`.
#[cfg(feature = "ssr")]
fn erro_rede(e: &reqwest::Error) -> ServerFnError {
    leptos::logging::error!("falha ao contatar a pcp-api: {e}");
    ServerFnError::new("Sem conexão com o servidor.")
}

/// Erro ao DESSERIALIZAR a resposta da `pcp-api`. O nome do campo que faltou é detalhe interno.
#[cfg(feature = "ssr")]
fn erro_resposta(e: &reqwest::Error) -> ServerFnError {
    leptos::logging::error!("resposta da pcp-api não pôde ser lida: {e}");
    ServerFnError::new("Não foi possível ler a resposta do servidor.")
}

#[cfg(feature = "ssr")]
use serde::Deserialize;

mod tipos;
pub use tipos::*;

/// Métricas do painel (`GET /pcp/dashboard`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = Painel, prefix = "/api")]
pub async fn painel(token: String) -> Result<PainelResumo, ServerFnError> {
    obter_json("/pcp/dashboard", &token).await
}

/// Resumo por classe (metas físicas + cobertura) — `GET /pcp/dashboard/classes`.
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = DashboardClasses, prefix = "/api")]
pub async fn dashboard_classes(token: String) -> Result<Vec<ClasseResumo>, ServerFnError> {
    obter_json("/pcp/dashboard/classes", &token).await
}

/// Série mensal de vendas (dado real) — `GET /pcp/dashboard/vendas-mensais`.
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = VendasMensais, prefix = "/api")]
pub async fn vendas_mensais(token: String) -> Result<Vec<VendaMes>, ServerFnError> {
    obter_json("/pcp/dashboard/vendas-mensais", &token).await
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
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Não foi possível carregar os produtos."));
    }
    resposta
        .json::<PaginaEstoque>()
        .await
        .map_err(|e| erro_resposta(&e))
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
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Não foi possível exportar."));
    }
    resposta.text().await.map_err(|e| erro_resposta(&e))
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
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Não foi possível salvar o filtro."));
    }
    resposta
        .json::<FiltroSalvo>()
        .await
        .map_err(|e| erro_resposta(&e))
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
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Não foi possível excluir o filtro."));
    }
    Ok(())
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
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if resposta.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Não foi possível carregar o produto."));
    }
    resposta
        .json::<DetalheProduto>()
        .await
        .map(Some)
        .map_err(|e| erro_resposta(&e))
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
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new(
            "Não foi possível carregar as solicitações.",
        ));
    }
    resposta
        .json::<Vec<Solicitacao>>()
        .await
        .map_err(|e| erro_resposta(&e))
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
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Não foi possível criar a solicitação."));
    }
    resposta
        .json::<Solicitacao>()
        .await
        .map_err(|e| erro_resposta(&e))
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
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if resposta.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(ServerFnError::new("Só o gestor pode alterar o estado."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new(
            "Não foi possível atualizar a solicitação.",
        ));
    }
    resposta
        .json::<Solicitacao>()
        .await
        .map_err(|e| erro_resposta(&e))
}

/// Tabela ABC completa (`GET /pcp/abc/tabela`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = AbcTabela, prefix = "/api")]
pub async fn abc_tabela(token: String) -> Result<Vec<LinhaAbc>, ServerFnError> {
    obter_json("/pcp/abc/tabela", &token).await
}

/// Distribuição ABC agregada (`GET /pcp/abc`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = AbcDistribuicao, prefix = "/api")]
pub async fn abc_distribuicao(token: String) -> Result<Vec<DistribuicaoAbc>, ServerFnError> {
    obter_json("/pcp/abc", &token).await
}

/// Fila de sugestões de ciclo de vida abertas (`GET /pcp/ciclo-vida`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = ListarCicloVida, prefix = "/api")]
pub async fn listar_ciclo_vida(token: String) -> Result<Vec<SugestaoCicloVida>, ServerFnError> {
    obter_json("/pcp/ciclo-vida", &token).await
}

/// Transiciona o estado de uma sugestão (`POST /pcp/ciclo-vida/{id}/transicao`) — gestor+.
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sem permissão ou transição inválida.
#[server(name = TransicionarCicloVida, prefix = "/api")]
pub async fn transicionar_ciclo_vida(
    token: String,
    id: String,
    para_estado: String,
) -> Result<SugestaoCicloVida, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .post(format!("{base}/pcp/ciclo-vida/{id}/transicao"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "para_estado": para_estado }))
        .send()
        .await
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if resposta.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(ServerFnError::new("Só o gestor pode aplicar."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Não foi possível atualizar a sugestão."));
    }
    resposta
        .json::<SugestaoCicloVida>()
        .await
        .map_err(|e| erro_resposta(&e))
}

/// Configuração de negócio vigente como JSON opaco (`GET /pcp/config`). O frontend não conhece a
/// regra (§3): só edita valores e devolve o documento. A validação é do servidor.
///
/// # Errors
/// [`ServerFnError`] em falha de rede ou sessão expirada.
#[server(name = ObterConfig, prefix = "/api")]
pub async fn obter_config(token: String) -> Result<serde_json::Value, ServerFnError> {
    obter_json("/pcp/config", &token).await
}

/// Salva a configuração (`PUT /pcp/config`) — gestor+. Servidor valida e recarrega a quente.
///
/// # Errors
/// [`ServerFnError`] sem permissão, config inválida (mensagem do servidor) ou falha de rede.
#[server(name = SalvarConfig, prefix = "/api")]
pub async fn salvar_config(
    token: String,
    config: serde_json::Value,
) -> Result<serde_json::Value, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .put(format!("{base}/pcp/config"))
        .bearer_auth(&token)
        .json(&config)
        .send()
        .await
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if resposta.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(ServerFnError::new(
            "apenas gestor pode editar a configuração",
        ));
    }
    if resposta.status() == reqwest::StatusCode::BAD_REQUEST {
        let msg = resposta.text().await.unwrap_or_default();
        return Err(ServerFnError::new(format!("configuração inválida: {msg}")));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new(
            "Não foi possível salvar a configuração.",
        ));
    }
    resposta
        .json::<serde_json::Value>()
        .await
        .map_err(|e| erro_resposta(&e))
}

/// Auditoria de configuração (`GET /pcp/config/auditoria`) — gestor+.
///
/// # Errors
/// [`ServerFnError`] sem permissão ou falha de rede.
#[server(name = AuditoriaConfig, prefix = "/api")]
pub async fn auditoria_config(token: String) -> Result<Vec<EntradaAuditoriaConfig>, ServerFnError> {
    obter_json("/pcp/config/auditoria", &token).await
}

/// Lista usuários (`GET /pcp/usuarios`) — admin.
///
/// # Errors
/// [`ServerFnError`] sem permissão ou falha de rede.
#[server(name = ListarUsuarios, prefix = "/api")]
pub async fn listar_usuarios(token: String) -> Result<Vec<UsuarioConta>, ServerFnError> {
    obter_json("/pcp/usuarios", &token).await
}

/// Cria usuário (`POST /pcp/usuarios`) — admin.
///
/// # Errors
/// [`ServerFnError`] sem permissão, dados inválidos ou e-mail já cadastrado.
#[server(name = CriarUsuario, prefix = "/api")]
pub async fn criar_usuario(
    token: String,
    email: String,
    senha: String,
    papel: String,
    nome: String,
) -> Result<(), ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let nome = (!nome.trim().is_empty()).then_some(nome);
    let resposta = reqwest::Client::new()
        .post(format!("{base}/pcp/usuarios"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "email": email, "senha": senha, "papel": papel, "nome": nome }))
        .send()
        .await
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(ServerFnError::new("Só o admin gerencia usuários."));
    }
    if resposta.status() == reqwest::StatusCode::CONFLICT {
        return Err(ServerFnError::new("Este e-mail já está cadastrado."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new(
            "falha ao criar usuário (verifique os dados)",
        ));
    }
    Ok(())
}

/// Atualiza papel/situação de um usuário (`PUT /pcp/usuarios/{id}`) — admin.
///
/// # Errors
/// [`ServerFnError`] sem permissão ou falha de rede.
#[server(name = AtualizarUsuario, prefix = "/api")]
pub async fn atualizar_usuario(
    token: String,
    id: String,
    papel: String,
    ativo: bool,
) -> Result<UsuarioConta, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .put(format!("{base}/pcp/usuarios/{id}"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "papel": papel, "ativo": ativo }))
        .send()
        .await
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(ServerFnError::new("Só o admin gerencia usuários."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Não foi possível atualizar o usuário."));
    }
    resposta
        .json::<UsuarioConta>()
        .await
        .map_err(|e| erro_resposta(&e))
}

/// Preferências do usuário (`GET /pcp/preferencias`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede ou sessão expirada.
#[server(name = ObterPreferencias, prefix = "/api")]
pub async fn obter_preferencias(token: String) -> Result<Preferencia, ServerFnError> {
    obter_json("/pcp/preferencias", &token).await
}

/// Salva preferências do usuário (`PUT /pcp/preferencias`).
///
/// # Errors
/// [`ServerFnError`] em valores inválidos ou falha de rede.
#[server(name = SalvarPreferencias, prefix = "/api")]
pub async fn salvar_preferencias(
    token: String,
    pagina_inicial: String,
    tamanho_pagina: i32,
) -> Result<Preferencia, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .put(format!("{base}/pcp/preferencias"))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "pagina_inicial": pagina_inicial,
            "tamanho_pagina": tamanho_pagina,
        }))
        .send()
        .await
        .map_err(|e| erro_rede(&e))?;
    if !resposta.status().is_success() {
        return Err(ServerFnError::new(
            "Não foi possível salvar as preferências.",
        ));
    }
    resposta
        .json::<Preferencia>()
        .await
        .map_err(|e| erro_resposta(&e))
}

/// Fatores sazonais vigentes (`GET /pcp/sazonalidade`).
///
/// # Errors
/// [`ServerFnError`] em falha de rede ou sessão expirada.
#[server(name = ListarSazonalidade, prefix = "/api")]
pub async fn listar_sazonalidade(token: String) -> Result<Vec<FatorMes>, ServerFnError> {
    obter_json("/pcp/sazonalidade", &token).await
}

/// Override manual do fator de um mês (`PUT /pcp/sazonalidade/{mes}`) — gestor.
///
/// # Errors
/// [`ServerFnError`] sem permissão, fator fora do intervalo ou falha de rede.
#[server(name = OverrideSazonalidade, prefix = "/api")]
pub async fn override_sazonalidade(
    token: String,
    mes: i16,
    fator: f64,
    justificativa: String,
) -> Result<FatorMes, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let justificativa = (!justificativa.trim().is_empty()).then_some(justificativa);
    let resposta = reqwest::Client::new()
        .put(format!("{base}/pcp/sazonalidade/{mes}"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "fator": fator, "justificativa": justificativa }))
        .send()
        .await
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(ServerFnError::new("Só o gestor edita a sazonalidade."));
    }
    if resposta.status() == reqwest::StatusCode::BAD_REQUEST {
        let msg = resposta.text().await.unwrap_or_default();
        return Err(ServerFnError::new(msg));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Não foi possível salvar o fator."));
    }
    resposta
        .json::<FatorMes>()
        .await
        .map_err(|e| erro_resposta(&e))
}

/// Insights estatísticos de um produto.
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = ProdutoInsights, prefix = "/api")]
pub async fn produto_insights(token: String, codigo: String) -> Result<Insights, ServerFnError> {
    obter_json(&format!("/pcp/produto/{codigo}/insights"), &token).await
}

/// Atividade de um produto: status/histórico de produção e movimentação (detalhe, doc 03 §4).
///
/// # Errors
/// [`ServerFnError`] em falha de rede, sessão expirada ou corpo inválido.
#[server(name = ProdutoAtividade, prefix = "/api")]
pub async fn produto_atividade(token: String, codigo: String) -> Result<Atividade, ServerFnError> {
    obter_json(&format!("/pcp/produto/{codigo}/atividade"), &token).await
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
        ("producao", c.producao.clone()),
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
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Não foi possível carregar os produtos."));
    }
    resposta.json::<T>().await.map_err(|e| erro_resposta(&e))
}

/// Execuções recentes do pipeline (`GET /pcp/admin/pipeline`, admin) — painel de operação.
///
/// # Errors
/// [`ServerFnError`] se a API não responder, a sessão expirar ou faltar permissão de admin.
#[server(name = AdminPipeline, prefix = "/api")]
pub async fn admin_pipeline(token: String) -> Result<Vec<ExecucaoPipeline>, ServerFnError> {
    obter_json("/pcp/admin/pipeline", &token).await
}

/// Health checks do pipeline/dados (`GET /pcp/admin/saude`, admin) — doc 05 §4.
///
/// # Errors
/// [`ServerFnError`] se a API não responder, a sessão expirar ou faltar permissão de admin.
#[server(name = AdminSaude, prefix = "/api")]
pub async fn admin_saude(token: String) -> Result<RelatorioSaude, ServerFnError> {
    obter_json("/pcp/admin/saude", &token).await
}

/// Dispara o reprocesso do pipeline para um intervalo (`POST /pcp/admin/reprocessar`, admin).
/// `inicio`/`fim` em `YYYY-MM-DD`. Devolve a mensagem de confirmação (processa em segundo plano).
///
/// # Errors
/// [`ServerFnError`] se a API não responder, faltar permissão ou o intervalo for inválido.
#[server(name = AdminReprocessar, prefix = "/api")]
pub async fn admin_reprocessar(
    token: String,
    inicio: String,
    fim: String,
) -> Result<String, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .post(format!("{base}/pcp/admin/reprocessar"))
        .bearer_auth(&token)
        .json(&serde_json::json!({ "inicio": inicio, "fim": fim }))
        .send()
        .await
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::FORBIDDEN {
        return Err(ServerFnError::new(
            "apenas administradores podem reprocessar",
        ));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new(
            "Intervalo inválido ou o reprocesso não pôde iniciar.",
        ));
    }
    let corpo: serde_json::Value = resposta.json().await.map_err(|e| erro_resposta(&e))?;
    Ok(corpo["mensagem"]
        .as_str()
        .unwrap_or("Reprocesso iniciado.")
        .to_owned())
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
        .map_err(|e| erro_rede(&e))?;
    if resposta.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Não foi possível carregar os alertas."));
    }
    resposta
        .json::<Vec<AlertaResumo>>()
        .await
        .map_err(|e| erro_resposta(&e))
}

/// Autentica na `pcp-api` (`POST /auth/login`) e devolve access + refresh token.
///
/// # Errors
/// [`ServerFnError`] se a API não responder ou as credenciais forem inválidas.
#[server(name = Login, prefix = "/api")]
pub async fn login(email: String, senha: String) -> Result<Credenciais, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .post(format!("{base}/auth/login"))
        .json(&serde_json::json!({ "email": email, "senha": senha }))
        .send()
        .await
        .map_err(|e| erro_rede(&e))?;
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("E-mail ou senha incorretos."));
    }
    let corpo: serde_json::Value = resposta.json().await.map_err(|e| erro_resposta(&e))?;
    let pegar = |chave: &str| corpo[chave].as_str().map(ToOwned::to_owned);
    match (pegar("access_token"), pegar("refresh_token")) {
        (Some(access_token), Some(refresh_token)) => Ok(Credenciais {
            access_token,
            refresh_token,
        }),
        _ => Err(ServerFnError::new(
            "Não foi possível entrar. Tente novamente.",
        )),
    }
}

/// Renova o `access_token` a partir de um refresh token salvo (`POST /auth/refresh`). Usado para
/// restaurar a sessão após reload. Erro = refresh inválido/expirado → o cliente cai no login.
///
/// # Errors
/// [`ServerFnError`] se a API não responder ou o refresh token não for válido.
#[server(name = Renovar, prefix = "/api")]
pub async fn renovar_sessao(refresh_token: String) -> Result<String, ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let resposta = reqwest::Client::new()
        .post(format!("{base}/auth/refresh"))
        .json(&serde_json::json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .map_err(|e| erro_rede(&e))?;
    if !resposta.status().is_success() {
        return Err(ServerFnError::new("Sessão expirada. Entre novamente."));
    }
    let corpo: serde_json::Value = resposta.json().await.map_err(|e| erro_resposta(&e))?;
    corpo["access_token"]
        .as_str()
        .map(ToOwned::to_owned)
        .ok_or_else(|| ServerFnError::new("Não foi possível entrar. Tente novamente."))
}

/// Revoga o refresh token no servidor (`POST /auth/logout`) — chamado ao sair.
///
/// # Errors
/// [`ServerFnError`] se a API não responder.
#[server(name = Logout, prefix = "/api")]
pub async fn encerrar_sessao(refresh_token: String) -> Result<(), ServerFnError> {
    let base = std::env::var("PCP_API_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    reqwest::Client::new()
        .post(format!("{base}/auth/logout"))
        .json(&serde_json::json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .map_err(|e| erro_rede(&e))?;
    Ok(())
}
