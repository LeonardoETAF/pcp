//! Endpoints de LEITURA sob `/pcp` (doc 04 §6.2). Cada handler lê valores já calculados pelo
//! motor (tabela `produto_ativo`/`alerta`) e os entrega como DTO — **sem recalcular regra**
//! (CLAUDE.md §3.2). Um handler por arquivo (§15).

pub mod abc;
pub mod alertas;
pub mod dashboard;
pub mod estoque;
pub mod estoque_exportacao;
pub mod eventos;
pub mod produto;
