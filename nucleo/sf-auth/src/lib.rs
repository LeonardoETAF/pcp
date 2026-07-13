//! **Núcleo do `SuperFlow` — autenticação e autorização.** Compartilhado por todos os módulos
//! (CLAUDE.md §0/§7): senha com argon2id, token de acesso JWT (HS256), refresh revogável
//! (só o hash é persistido) e middleware deny-by-default.
//!
//! O middleware é **genérico no estado da aplicação**: qualquer módulo o usa desde que o seu
//! `AppState` saiba entregar o [`SegredoJwt`] (via `FromRef` do Axum). É assim que o Catálogo
//! autentica sem conhecer o PCP — nenhum módulo depende de outro (§0).
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

mod jwt;
mod middleware;
mod papel;
mod refresh;
mod senha;

pub use jwt::{decodificar, gerar_access, Claims};
pub use middleware::{exigir_autenticacao, SegredoJwt};
pub use papel::Papel;
pub use refresh::{gerar_refresh, hash_refresh};
pub use senha::{hashear, verificar};
