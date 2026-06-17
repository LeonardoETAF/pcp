//! Backfill inicial a partir do ERP One (somente-leitura) para o banco do PCP.
//!
//! Lê `ONE_DATABASE_URL` (fonte legada) e `DATABASE_URL` (PCP) do ambiente/.env — segredos
//! nunca no código (CLAUDE.md §7.4). Ingere as vendas dos últimos N meses (pedidos não
//! cancelados) e o snapshot de estoque do dia, de forma idempotente por dia (doc 05 §2.3).
//! Só ingere; o motor roda à parte (bin `processar` do pcp-engine).
//!
//! Uso: `cargo run -p pcp-etl --bin backfill_one` (janela via `ONE_BACKFILL_MESES`, padrão 24).
#![forbid(unsafe_code)]

use chrono::{Local, Months};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let one_url = std::env::var("ONE_DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("defina ONE_DATABASE_URL no ambiente/.env"))?;
    let pcp_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("defina DATABASE_URL no ambiente/.env"))?;
    let meses: u32 = std::env::var("ONE_BACKFILL_MESES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24);

    let hoje = Local::now().date_naive();
    let desde = hoje
        .checked_sub_months(Months::new(meses))
        .ok_or_else(|| anyhow::anyhow!("janela de {meses} meses inválida"))?;

    eprintln!("• Conectando ao One (somente-leitura)…");
    let fonte = pcp_etl::FonteConsultaOne::conectar(&one_url, 2).await?;
    let pcp = pcp_db::criar_pool(&pcp_url, 5).await?;

    eprintln!("• Lendo vendas desde {desde} (pedidos não cancelados, produto acabado)…");
    let vendas = fonte.ler_vendas(desde).await?;
    eprintln!("  {} linhas de venda agregadas", vendas.len());

    eprintln!("• Lendo snapshot de estoque ({hoje})…");
    let snapshot = fonte.ler_snapshot(hoje).await?;
    eprintln!("  {} produtos no snapshot", snapshot.len());

    eprintln!("• Gravando no PCP (idempotente por dia)…");
    let r = pcp_etl::gravar(&pcp, vendas, snapshot).await?;

    println!(
        "Backfill concluído: vendas {} dias / {} linhas · snapshot {} dias / {} linhas",
        r.dias_vendas, r.linhas_vendas, r.dias_snapshot, r.linhas_snapshot
    );
    Ok(())
}
