//! Contextos compartilhados do frontend (sessão e tema). Sem regra de negócio (CLAUDE.md §3).

use leptos::prelude::*;

/// Sessão do usuário: `access_token` em memória. Após reload, é restaurado a partir do refresh
/// token salvo (ver `armazenamento::REFRESH`). Endurecimento futuro = cookie httpOnly (§3/§7).
#[derive(Clone, Copy)]
pub struct Sessao(pub RwSignal<Option<String>>);

/// Indica que a restauração da sessão (via refresh token salvo) está em andamento. Evita que o
/// layout redirecione ao login antes de tentar restaurar — corrige o "voltar ao login" no reload.
#[derive(Clone, Copy)]
pub struct CarregandoSessao(pub RwSignal<bool>);

/// Tema visual atual (`"claro"` | `"escuro"`), aplicado via `data-tema` na raiz da app (§16).
#[derive(Clone, Copy)]
pub struct Tema(pub RwSignal<&'static str>);

/// Estado da lista de estoque (busca, filtros, ordenação e página). Vive no CONTEXTO, não na
/// página: sair para o detalhe de um produto e voltar não pode perder a pesquisa nem a página em
/// que o usuário estava — a página é remontada, mas estes sinais sobrevivem (§16, adaptatividade).
#[derive(Clone, Copy)]
pub struct FiltroEstoque {
    pub classe: RwSignal<Option<String>>,
    pub status: RwSignal<Option<String>>,
    /// Termo já aplicado (o que filtra a consulta).
    pub busca: RwSignal<String>,
    /// O que está sendo digitado no campo (aplicado no Enter).
    pub busca_input: RwSignal<String>,
    pub ordem: RwSignal<String>,
    /// Estado de produção: `em_producao` | `aguardando` | `recem_produzido`.
    pub producao: RwSignal<Option<String>>,
    pub limite: RwSignal<i64>,
    pub deslocamento: RwSignal<i64>,
}

impl Default for FiltroEstoque {
    fn default() -> Self {
        Self {
            classe: RwSignal::new(None),
            status: RwSignal::new(None),
            busca: RwSignal::new(String::new()),
            busca_input: RwSignal::new(String::new()),
            ordem: RwSignal::new("sugerida_desc".to_owned()),
            producao: RwSignal::new(None),
            limite: RwSignal::new(50),
            deslocamento: RwSignal::new(0),
        }
    }
}
