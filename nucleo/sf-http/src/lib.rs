//! **Núcleo do `SuperFlow` — HTTP.** Erro de API compartilhado por todos os módulos, com o
//! mapeamento canônico para status HTTP. Mensagens genéricas: nunca vazam detalhe interno
//! (CLAUDE.md §7).
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

mod erro;

pub use erro::ApiError;
