//! Expurgo de retenção (CLAUDE.md §9/§13): apaga dados que excedem a janela de
//! `pcp.retencao_politica` e remove refresh tokens vencidos. Idempotente. Pensado para rodar
//! periodicamente (ex.: diariamente via cron na operação — Fase 5). Lê `DATABASE_URL` do ambiente.
//!
//! Uso: `cargo run -p pcp-engine --bin expurgar`
#![forbid(unsafe_code)]

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("defina DATABASE_URL no ambiente/.env"))?;
    let pool = pcp_db::criar_pool(&url, 5).await?;

    let resultado = pcp_db::expurgar(&pool).await?;
    let total: u64 = resultado.iter().map(|(_, n)| n).sum();
    for (dataset, n) in &resultado {
        println!("{dataset}: {n} linhas removidas");
    }
    println!("Expurgo concluído: {total} linhas removidas no total.");
    Ok(())
}
