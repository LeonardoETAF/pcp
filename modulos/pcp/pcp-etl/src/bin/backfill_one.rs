//! Backfill inicial a partir do ERP One (somente-leitura) para o banco do PCP.
//!
//! Lê `ONE_DATABASE_URL` (fonte legada) e `DATABASE_URL` (PCP) do ambiente/.env — segredos
//! nunca no código (CLAUDE.md §7.4). Ingere as vendas dos últimos N meses (pedidos não
//! cancelados) e o snapshot de estoque do dia, de forma idempotente por dia (doc 05 §2.3).
//! Só ingere; o motor roda à parte (bin `processar` do pcp-engine).
//!
//! Uso: `cargo run -p pcp-etl --bin backfill_one` (janela via `ONE_BACKFILL_MESES`, padrão 24).
//! Na 1ª execução faz o backfill da janela inteira; nas seguintes, o `FonteConsultaOne` lê só o
//! incremental (marca-d'água) — então este bin serve tanto p/ carga inicial quanto p/ recarga.
#![forbid(unsafe_code)]

use chrono::Local;

use pcp_etl::{FonteConsultaOne, OpcoesOne};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let one_url = std::env::var("ONE_DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("defina ONE_DATABASE_URL no ambiente/.env"))?;
    let pcp_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("defina DATABASE_URL no ambiente/.env"))?;
    let meses: i64 = std::env::var("ONE_BACKFILL_MESES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(24);

    let pcp = pcp_db::criar_pool(&pcp_url, 5).await?;
    let opcoes = OpcoesOne {
        data_ref: Local::now().date_naive(),
        backfill_dias: meses * 31, // folga sobre 24 meses (ABC usa 18m; sazonal, ano anterior)
        janela_deslizante_dias: 15,
    };

    eprintln!("• Conectando ao One (somente-leitura) e LANDando o cru no bronze…");
    let fonte = FonteConsultaOne::conectar(&one_url, pcp.clone(), opcoes).await?;

    eprintln!("• Ingerindo (bronze → ACL → domínio, idempotente por dia)…");
    let r = pcp_etl::importar(&pcp, &fonte).await?;

    eprintln!("• Sincronizando complementares (faturada + produção) no bronze…");
    let desde = opcoes.data_ref - chrono::Duration::days(opcoes.backfill_dias);
    let (faturas, producao) = fonte.sincronizar_complementares(desde).await?;
    eprintln!("  {faturas} faturas, {producao} itens de produção");

    println!(
        "Ingestão concluída: vendas {} dias / {} linhas · snapshot {} dias / {} linhas",
        r.dias_vendas, r.linhas_vendas, r.dias_snapshot, r.linhas_snapshot
    );
    Ok(())
}
