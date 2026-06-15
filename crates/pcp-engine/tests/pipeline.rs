//! Teste de integração do motor diário (precisa de Postgres — `DATABASE_URL`). `#[ignore]`.
//! Rode com: `docker compose up -d` e `cargo test -p pcp-engine -- --ignored`.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

use std::path::PathBuf;

use chrono::NaiveDate;

use pcp_config::Config;
use pcp_db::{snapshot, vendas, NovaVendaDia, NovoEstoqueSnapshot, PgPool};
use pcp_engine::{processar_dia, StatusPipeline};

fn data_ref() -> NaiveDate {
    NaiveDate::from_ymd_opt(2099, 6, 15).expect("data válida")
}

async fn preparar() -> (PgPool, Config) {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL para os testes de banco");
    let pool = pcp_db::criar_pool(&url, 5).await.expect("pool");
    pcp_db::aplicar_migrations(&pool).await.expect("migrations");

    let caminho = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config/pcp.config.yaml");
    let config = pcp_config::carregar_de_arquivo(caminho).expect("config de referência");

    limpar(&pool).await;
    (pool, config)
}

/// Remove os dados de teste (produtos PIPE-% e a data 2099-06-15) para não vazar entre testes.
async fn limpar(pool: &PgPool) {
    for sql in [
        "DELETE FROM pcp.classificacao WHERE codigo_estoque LIKE 'PIPE-%'",
        "DELETE FROM pcp.alerta WHERE codigo_estoque LIKE 'PIPE-%'",
        "DELETE FROM pcp.estoque_param WHERE codigo_estoque LIKE 'PIPE-%'",
        "DELETE FROM pcp.sugestao_ciclo_vida WHERE codigo_estoque LIKE 'PIPE-%'",
        "DELETE FROM pcp.vendas_dia WHERE codigo_estoque LIKE 'PIPE-%'",
        "DELETE FROM pcp.estoque_snapshot WHERE codigo_estoque LIKE 'PIPE-%'",
        "DELETE FROM pcp.produto_ativo WHERE codigo_estoque LIKE 'PIPE-%'",
        "DELETE FROM pcp.execucao_pipeline WHERE data_ref = DATE '2099-06-15'",
    ] {
        sqlx::query(sql).execute(pool).await.expect("limpeza");
    }
}

fn venda(dia: NaiveDate, codigo: &str, qtd: i32) -> NovaVendaDia {
    NovaVendaDia {
        dt_ref: dia,
        codigo_estoque: codigo.to_owned(),
        sku: None,
        produto: Some("Copo".to_owned()),
        configuracao: None,
        qtd_vendida: qtd,
        is_personalizado: false,
    }
}

fn snap(codigo: &str, estoque: i32, reserva: i32, fora: bool) -> NovoEstoqueSnapshot {
    NovoEstoqueSnapshot {
        dt_ref: data_ref(),
        codigo_estoque: codigo.to_owned(),
        sku: None,
        produto: Some("Copo".to_owned()),
        configuracao: None,
        qtd_estoque: estoque,
        qtd_reserva: reserva,
        qtd_disponivel: estoque - reserva,
        estoque_min_erp: None,
        fora_de_linha: fora,
    }
}

async fn conta(pool: &PgPool, sql: &'static str) -> i64 {
    sqlx::query_scalar::<_, i64>(sql)
        .fetch_one(pool)
        .await
        .expect("contagem")
}

#[tokio::test]
#[ignore = "precisa de Postgres (DATABASE_URL); rode com --ignored"]
async fn pipeline_completo_idempotente_e_bloqueio() {
    let (pool, config) = preparar().await;
    let dia = data_ref();

    // PIPE-A: maduro, vende em 12 dias + véspera (classe Pareto, com parâmetros).
    for d in 2..=13 {
        let venda_dia = NaiveDate::from_ymd_opt(2099, 1, d).unwrap();
        vendas::substituir_dia(&pool, venda_dia, &[venda(venda_dia, "PIPE-A", 10)])
            .await
            .unwrap();
    }
    let vespera = NaiveDate::from_ymd_opt(2099, 6, 14).unwrap(); // pré-validação (dia anterior)
    vendas::substituir_dia(&pool, vespera, &[venda(vespera, "PIPE-A", 10)])
        .await
        .unwrap();

    // Snapshot do dia: PIPE-A (ativo, baixo), PIPE-D (ativo sem vendas → D/SAIR), PIPE-F (fora).
    snapshot::substituir_dia(
        &pool,
        dia,
        &[
            snap("PIPE-A", 50, 10, false),
            snap("PIPE-D", 100, 0, false),
            snap("PIPE-F", 5, 0, true),
        ],
    )
    .await
    .unwrap();

    // 1ª execução.
    let res = processar_dia(&pool, &config, dia).await.unwrap();
    assert_eq!(res.status, StatusPipeline::Completo);
    assert_eq!(res.execucoes.len(), 5);
    assert!(res.execucoes.iter().all(|e| e.status == "sucesso"));

    // 3 produtos classificados (PIPE-A, PIPE-D, PIPE-F).
    assert_eq!(
        conta(
            &pool,
            "SELECT COUNT(*) FROM pcp.classificacao WHERE dt_calculo = DATE '2099-06-15'"
        )
        .await,
        3
    );
    assert_eq!(
        conta(
            &pool,
            "SELECT COUNT(*) FROM pcp.estoque_param WHERE codigo_estoque LIKE 'PIPE-%'"
        )
        .await,
        3
    );
    // PIPE-A gera alerta; PIPE-D (sem histórico) e PIPE-F (fora) não.
    assert!(
        conta(
            &pool,
            "SELECT COUNT(*) FROM pcp.alerta WHERE dt_alerta = DATE '2099-06-15'"
        )
        .await
            >= 1
    );
    // PIPE-D ativo sem vendas -> sugestão SAIR (gerada).
    assert_eq!(conta(&pool, "SELECT COUNT(*) FROM pcp.sugestao_ciclo_vida WHERE codigo_estoque = 'PIPE-D' AND estado = 'gerada'").await, 1);
    // 5 módulos registrados na telemetria (incl. consolidação).
    assert_eq!(
        conta(
            &pool,
            "SELECT COUNT(*) FROM pcp.execucao_pipeline WHERE data_ref = DATE '2099-06-15'"
        )
        .await,
        5
    );
    // Consolidação: os 3 produtos na "view" materializada produto_ativo (doc 04 §5).
    assert_eq!(
        conta(
            &pool,
            "SELECT COUNT(*) FROM pcp.produto_ativo WHERE codigo_estoque LIKE 'PIPE-%'"
        )
        .await,
        3
    );
    // PIPE-F é fora de linha -> status canônico 'fora_de_linha' (doc 02 §5.2).
    assert_eq!(
        conta(
            &pool,
            "SELECT COUNT(*) FROM pcp.produto_ativo WHERE codigo_estoque = 'PIPE-F' AND status = 'fora_de_linha'"
        )
        .await,
        1
    );

    // 2ª execução (idempotência): contagens das derivadas estáveis.
    let res2 = processar_dia(&pool, &config, dia).await.unwrap();
    assert_eq!(res2.status, StatusPipeline::Completo);
    assert_eq!(
        conta(
            &pool,
            "SELECT COUNT(*) FROM pcp.classificacao WHERE dt_calculo = DATE '2099-06-15'"
        )
        .await,
        3
    );
    assert_eq!(conta(&pool, "SELECT COUNT(*) FROM pcp.sugestao_ciclo_vida WHERE codigo_estoque = 'PIPE-D' AND estado = 'gerada'").await, 1);

    // Pré-validação bloqueante: data sem dados -> Bloqueado.
    let vazio = NaiveDate::from_ymd_opt(2098, 3, 3).unwrap();
    let res_bloq = processar_dia(&pool, &config, vazio).await.unwrap();
    assert_eq!(res_bloq.status, StatusPipeline::Bloqueado);
    assert!(res_bloq.execucoes.is_empty());

    // Limpa ao final para não vazar o snapshot de 2099-06-15 para outros testes.
    limpar(&pool).await;
}
