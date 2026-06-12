//! Motor diário: orquestra classificação → parâmetros → alertas → fora de linha, de forma
//! idempotente por `data_ref`, com isolamento de falha por módulo. (CLAUDE.md §3.4/§8)
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

// Esqueleto: o orquestrador diário entra no prompt 1.6.
