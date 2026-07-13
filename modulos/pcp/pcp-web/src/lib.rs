//! Frontend Leptos (SSR + hidratação WASM): burro em regra, só exibe valores prontos da
//! API. Build por cargo-leptos; estilo em CSS à mão com design tokens (CLAUDE.md §1/§16).
//!
//! EXCEÇÃO documentada (CLAUDE.md §3/§5, autorizada pelo dono em 2026-06-16): este crate usa
//! `#![deny(unsafe_code)]` em vez de `forbid`. Motivo: o ponto de hidratação WASM é gerado pela
//! macro `#[wasm_bindgen]`, que emite `unsafe` inevitável em qualquer frontend Rust→WASM. O
//! único `allow(unsafe_code)` do projeto fica isolado no módulo `hydratacao`. Nenhum `unsafe`
//! escrito à mão; todos os demais crates mantêm `#![forbid(unsafe_code)]`.
#![deny(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]
// Componentes Leptos retornam `impl IntoView` consumido pela macro `view!`; o `must_use` do
// pedantic vira ruído em todo componente. Relaxado só este sub-lint (CLAUDE.md §5).
#![allow(clippy::must_use_candidate)]
// As `view!` aninhadas (layout/páginas) geram tipos profundos; o release resolve o layout
// completo e estoura o limite padrão (128). Sobe o limite (não afeta runtime).
#![recursion_limit = "512"]

pub mod api;
pub mod app;
mod armazenamento;
mod componentes;
mod contexto;
mod download;
mod erro;
mod formato;
mod hydratacao;
mod layout;
mod paginas;
