//! Smoke test do esqueleto do workspace — garante que a infraestrutura de testes roda.
//! Será acompanhado dos testes de paridade/invariantes (prompts 1.1 e 1.7).
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

#[test]
fn workspace_executa_testes() {
    let soma: u32 = (1..=3).sum();
    assert_eq!(soma, 6);
}
