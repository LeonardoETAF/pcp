//! Sincronização contínua com o ERP One (polling quase-tempo-real — CLAUDE.md §16, doc 05).
//!
//! A cada `ONE_POLL_SEGUNDOS` (padrão 300): ingere do One (estoque full refresh + vendas
//! incremental com janela deslizante, via `FonteConsultaOne`) e roda o pipeline do dia. O
//! `processar_dia` notifica o canal LISTEN/NOTIFY ao terminar — a `pcp-api` já escuta e repassa
//! por SSE, então a UI atualiza sozinha quando chega dado novo. Falha de um ciclo é registrada e
//! o loop continua (resiliência operacional). Idempotente por dia.
//!
//! Uso: `cargo run -p pcp-etl --bin sync_one` (Ctrl-C para encerrar).
#![forbid(unsafe_code)]

use std::time::Duration;

use chrono::Local;

use pcp_etl::{FonteConsultaOne, OpcoesOne};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let one_url = std::env::var("ONE_DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("defina ONE_DATABASE_URL no ambiente/.env"))?;
    let pcp_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow::anyhow!("defina DATABASE_URL no ambiente/.env"))?;
    let intervalo = Duration::from_secs(
        std::env::var("ONE_POLL_SEGUNDOS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(300),
    );
    let backfill_dias: i64 = std::env::var("ONE_BACKFILL_MESES")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(24)
        * 31;
    let caminho_config =
        std::env::var("PCP_CONFIG_PATH").unwrap_or_else(|_| "config/pcp.config.yaml".to_owned());

    let pcp = pcp_db::criar_pool(&pcp_url, 5).await?;
    let config = pcp_config::carregar_de_arquivo(&caminho_config)?;

    eprintln!(
        "• Sincronização One iniciada (intervalo {}s). Ctrl-C para encerrar.",
        intervalo.as_secs()
    );
    loop {
        match ciclo(&one_url, &pcp, &config, backfill_dias).await {
            Ok(()) => {}
            // Resiliência: um ciclo que falha (rede, lock, dado) não derruba o serviço.
            Err(e) => eprintln!("! ciclo falhou ({e}); tentando no próximo intervalo"),
        }
        tokio::time::sleep(intervalo).await;
    }
}

/// Um ciclo: conecta ao One, ingere (bronze → ACL → domínio) e roda o pipeline do dia.
async fn ciclo(
    one_url: &str,
    pcp: &pcp_db::PgPool,
    config: &pcp_config::Config,
    backfill_dias: i64,
) -> anyhow::Result<()> {
    let hoje = Local::now().date_naive();
    let opcoes = OpcoesOne {
        data_ref: hoje,
        backfill_dias,
        janela_deslizante_dias: 15,
    };
    let fonte = FonteConsultaOne::conectar(one_url, pcp.clone(), opcoes).await?;
    let r = pcp_etl::importar(pcp, &fonte).await?;
    let resultado = pcp_engine::processar_dia(pcp, config, hoje).await?;
    eprintln!(
        "• {hoje}: vendas {} / snapshot {} linhas · pipeline {:?} (notificado → SSE)",
        r.linhas_vendas, r.linhas_snapshot, resultado.status
    );
    // Complementares (faturada/produção) — janela recente; best-effort (não abortam o ciclo).
    match fonte
        .sincronizar_complementares(hoje - chrono::Duration::days(30))
        .await
    {
        Ok((f, p)) => eprintln!("  complementares: {f} faturas, {p} itens de produção"),
        Err(e) => eprintln!("  (complementares falharam: {e})"),
    }
    Ok(())
}
