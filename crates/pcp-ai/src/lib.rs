//! IA: chat com tool use (somente leitura), análise por produto com fallback local e
//! insights estatísticos no backend, via Anthropic Claude. (CLAUDE.md §10)
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

// Esqueleto: insights, chat e análise por produto entram nos prompts 4.1–4.3.
