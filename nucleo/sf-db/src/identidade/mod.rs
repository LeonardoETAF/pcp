//! Identidade do `SuperFlow` (schema `nucleo`): usuários e refresh tokens.
//! Persistência pura — a regra de papéis/auth vive no `sf-auth` (CLAUDE.md §7).

pub mod refresh_tokens;
pub mod usuarios;

pub use refresh_tokens::RefreshToken;
pub use usuarios::Usuario;
