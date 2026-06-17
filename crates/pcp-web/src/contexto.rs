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
