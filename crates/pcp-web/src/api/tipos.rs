//! Tipos de dados (DTOs) trocados com a `pcp-api` — espelham os DTOs do servidor.
//! Dados puros, sem regra de negócio (CLAUDE.md §3); reexportados por `super`.

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
    /// Quantidade por classe sob o filtro atual (busca/status), ignorando a classe escolhida.
    #[serde(default)]
    pub contagem_classes: Vec<ContagemClasse>,
}

/// Contagem de uma classe nas abas da tela de estoque.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ContagemClasse {
    pub classe: String,
    pub quantidade: i64,
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
    /// Meta de cobertura da classe em dias (config §11) — base do anel de cobertura.
    pub cobertura_meta_dias: u32,
}

/// Total de vendas de um mês (série do dashboard, dado real de `vendas_dia`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VendaMes {
    pub ano: i32,
    pub mes: i32,
    pub total: i64,
}

/// Filtro salvo do usuário (`/pcp/estoque/filtros`). `filtro` é o JSON opaco de [`ConsultaEstoque`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FiltroSalvo {
    pub id: String,
    pub nome: String,
    pub filtro: serde_json::Value,
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

/// Linha da tabela de Classificação ABC (`GET /pcp/abc/tabela`) — 1 por produto, mais recente.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LinhaAbc {
    pub codigo_estoque: String,
    pub produto: Option<String>,
    pub classe: String,
    pub volume_janela: i64,
    pub percentual_acumulado: Option<f64>,
    pub fator_estoque: f64,
    pub estoque_atual: i64,
    pub status: String,
}

/// Sugestão de ciclo de vida / fora de linha (`/pcp/ciclo-vida`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SugestaoCicloVida {
    pub id: String,
    pub codigo_estoque: String,
    pub acao_sugerida: String,
    pub pontuacao: i16,
    pub nivel_certeza: String,
    pub criterios: Vec<String>,
    pub estado: String,
    pub data_analise: String,
    pub aplicado_por: Option<String>,
    pub observacoes: Option<String>,
}

/// Distribuição por classe ABC (`GET /pcp/abc`) — agregação feita no banco (§15: nunca no
/// cliente). 1 linha por classe presente, com contagem/volume/recomendado.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DistribuicaoAbc {
    pub classe: String,
    pub quantidade: i64,
    pub volume: i64,
    pub recomendado: i64,
}

/// Uma entrada da auditoria de configuração (`GET /pcp/config/auditoria`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EntradaAuditoriaConfig {
    pub chave: String,
    pub valor_anterior: Option<String>,
    pub valor_novo: Option<String>,
    pub por_id: String,
    pub em: String,
}

/// Usuário para a tela de gestão (`/pcp/usuarios`). Sem `senha_hash` (nunca trafega).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct UsuarioConta {
    pub id: String,
    pub email: String,
    pub papel: String,
    pub nome: Option<String>,
    pub ativo: bool,
}

/// Preferências de exibição do usuário (`/pcp/preferencias`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Preferencia {
    pub pagina_inicial: String,
    pub tamanho_pagina: i32,
}

/// Fator sazonal de um mês (`/pcp/sazonalidade`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FatorMes {
    pub mes: i16,
    pub fator: f64,
}

/// Alerta inteligente dos insights estatísticos (doc 06 §3.3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlertaInteligente {
    pub categoria: String,
    pub severidade: String,
    pub titulo: String,
    pub detalhe: String,
}

/// Insights estatísticos do produto (`GET /pcp/produto/{codigo}/insights`). Tudo já calculado no
/// backend pelo motor `pcp-ai` (frontend burro — §3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Insights {
    pub slope: f64,
    pub correlacao: f64,
    pub baseline_ma7: f64,
    pub forca_sazonal: f64,
    pub fatores_dia: [f64; 7],
    pub previsao_7d: Vec<f64>,
    pub total_previsto_7d: f64,
    pub total_previsto_30d: f64,
    pub confianca: f64,
    pub dias_com_venda_pct: f64,
    pub alertas: Vec<AlertaInteligente>,
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

/// Credenciais devolvidas pelo login: `access_token` (curto, fica em memória) + `refresh_token`
/// (longo, persistido no cliente para restaurar a sessão após reload — ver `contexto::Sessao`).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credenciais {
    pub access_token: String,
    pub refresh_token: String,
}

/// Execução de um módulo do pipeline (`GET /pcp/admin/pipeline`) — painel de operação (doc 05 §3).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExecucaoPipeline {
    pub data_ref: String,
    pub modulo: String,
    pub status: String,
    pub linhas_afetadas: i64,
    pub duracao_ms: i64,
    pub erro: Option<String>,
    pub inicio: String,
    pub fim: String,
}

/// Uma verificação de saúde (doc 05 §4) com veredito pronto da API: `status` ∈ ok|atencao|critico.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificacaoSaude {
    pub nome: String,
    pub status: String,
    pub detalhe: String,
}

/// Relatório de health checks (`GET /pcp/admin/saude`) — doc 05 §4.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RelatorioSaude {
    pub gerado_em: String,
    pub verificacoes: Vec<VerificacaoSaude>,
}

/// Situação atual da produção da linha (detalhe do produto, doc 03 §4).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StatusProducao {
    pub ordens_abertas: i64,
    pub qtd_planejada: i64,
    pub em_producao: i64,
    pub aguardando: i64,
}

/// Uma ordem de produção da linha.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OrdemProducao {
    pub data: Option<String>,
    pub quantidade: i64,
    pub status: Option<String>,
    pub lote: Option<i64>,
}

/// Um movimento de estoque (kardex) da linha.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Movimento {
    pub data: String,
    pub tipo: String,
    pub quantidade: i64,
    pub saldo: i64,
}

/// Atividade da linha de estoque: status/histórico de produção e histórico de movimentação.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Atividade {
    pub status_producao: StatusProducao,
    pub producao: Vec<OrdemProducao>,
    pub movimentos: Vec<Movimento>,
}
