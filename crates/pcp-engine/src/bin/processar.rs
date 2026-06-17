//! Roda o pipeline diário para uma `data_ref` (CLAUDE.md §3, doc 05 §1.2) — idempotente.
//!
//! Lê `DATABASE_URL` do ambiente/.env (§7.4) e a configuração de `config/pcp.config.yaml`
//! (ou de `PCP_CONFIG_PATH`). A regra de negócio vive no `pcp-core`; aqui é só orquestração.
//!
//! Uso: `cargo run -p pcp-engine --bin processar -- 2026-06-17`
#![forbid(unsafe_code)]

use chrono::NaiveDate;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let data: NaiveDate = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("uso: processar <YYYY-MM-DD>"))?
        .parse()
        .map_err(|_| anyhow::anyhow!("data inválida (esperado YYYY-MM-DD)"))?;

    let url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("defina DATABASE_URL no ambiente/.env"))?;
    let caminho_config =
        std::env::var("PCP_CONFIG_PATH").unwrap_or_else(|_| "config/pcp.config.yaml".to_owned());

    let pool = pcp_db::criar_pool(&url, 5).await?;
    let config = pcp_config::carregar_de_arquivo(&caminho_config)?;

    eprintln!("• Processando {data}…");
    let resultado = pcp_engine::processar_dia(&pool, &config, data).await?;
    println!("{resultado:#?}");
    Ok(())
}
