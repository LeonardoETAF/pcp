//! Contextos compartilhados do frontend (sessão e tema). Sem regra de negócio (CLAUDE.md §3).

use leptos::prelude::*;

/// Sessão do usuário: `access_token` em memória. Persistência/cookie httpOnly = endurecimento
/// futuro de produção. O frontend só guarda o token devolvido pela API (§3/§7).
#[derive(Clone, Copy)]
pub struct Sessao(pub RwSignal<Option<String>>);

/// Tema visual atual (`"claro"` | `"escuro"`), aplicado via `data-tema` na raiz da app (§16).
#[derive(Clone, Copy)]
pub struct Tema(pub RwSignal<&'static str>);
