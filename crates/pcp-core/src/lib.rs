//! Domínio puro do PCP: todas as regras do doc 02 (classificação, parâmetros, alertas,
//! ciclo de vida, sazonalidade). Funções puras, sem relógio/banco/rede. (CLAUDE.md §2/§3)
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

// Esqueleto: as regras de negócio entram a partir do prompt 1.1.
