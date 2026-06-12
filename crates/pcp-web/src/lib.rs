//! Frontend Leptos (SSR + hidratação WASM): burro em regra, só exibe valores prontos da
//! API. Build por cargo-leptos/Trunk; estilo em CSS à mão (design tokens). (CLAUDE.md §1/§16)
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

// Esqueleto: a fundação do frontend (design system, shell, login) entra no prompt 2.2.
