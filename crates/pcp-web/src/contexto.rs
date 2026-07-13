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

/// Um produto marcado na lista para virar Solicitação de Produção (doc 02 §7.2).
/// Guardamos o essencial (não o código só) para a tela de criação não precisar rebuscar a API.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProdutoSelecionado {
    pub codigo: String,
    pub nome: String,
    pub qtd_sugerida: i64,
}

/// Produtos que o funcionário do PCP marcou na lista para produzir. Vive no contexto: a seleção
/// atravessa páginas da lista, filtros e a ida ao detalhe de um produto.
#[derive(Clone, Copy, Default)]
pub struct SelecaoProducao(pub RwSignal<Vec<ProdutoSelecionado>>);

impl SelecaoProducao {
    /// Marca/desmarca um produto.
    pub fn alternar(self, p: ProdutoSelecionado) {
        self.0.update(|v| {
            if let Some(i) = v.iter().position(|x| x.codigo == p.codigo) {
                v.remove(i);
            } else {
                v.push(p);
            }
        });
    }

    /// O produto está marcado?
    #[must_use]
    pub fn tem(self, codigo: &str) -> bool {
        self.0.read().iter().any(|x| x.codigo == codigo)
    }

    /// Limpa a seleção.
    pub fn limpar(self) {
        self.0.update(std::vec::Vec::clear);
    }
}
