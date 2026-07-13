//! Integração ETL → motor (precisa de Postgres — `DATABASE_URL`). `#[ignore]`.
//! Importa um dump CSV sintético e roda o motor sobre a data, ponta a ponta.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

use std::path::PathBuf;

use chrono::NaiveDate;

use pcp_config::Config;
use pcp_db::PgPool;
use pcp_engine::{processar_dia, StatusPipeline};
use pcp_etl::{importar, ImportadorArquivo};

const VENDAS: &str =
    "dt_ref,codigo_estoque,sku,produto,configuracao,qtd_vendida,is_personalizado\n\
2099-07-14,ETL-A,SKU-A,Copo,COR DO PRODUTO: AZUL,10,false\n\
2099-07-14,ETL-A,SKU-A,Copo,,4,true\n";

const SNAPSHOT: &str = "dt_ref,codigo_estoque,sku,produto,configuracao,qtd_estoque,qtd_reserva,qtd_disponivel,estoque_min_erp,fora_de_linha\n\
2099-07-15,ETL-A,SKU-A,Copo,,50,10,40,,false\n";

async fn preparar() -> (PgPool, Config) {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL para os testes de banco");
    let pool = pcp_db::criar_pool(&url, 5).await.expect("pool");
    pcp_db::aplicar_migrations(&pool).await.expect("migrations");
    let config = pcp_config::carregar_de_arquivo(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../config/pcp.config.yaml"),
    )
    .expect("config de referência");
    limpar(&pool).await;
    (pool, config)
}

async fn limpar(pool: &PgPool) {
    for sql in [
        "DELETE FROM pcp.classificacao WHERE codigo_estoque LIKE 'ETL-%'",
        "DELETE FROM pcp.alerta WHERE codigo_estoque LIKE 'ETL-%'",
        "DELETE FROM pcp.estoque_param WHERE codigo_estoque LIKE 'ETL-%'",
        "DELETE FROM pcp.sugestao_ciclo_vida WHERE codigo_estoque LIKE 'ETL-%'",
        "DELETE FROM pcp.vendas_dia WHERE codigo_estoque LIKE 'ETL-%'",
        "DELETE FROM pcp.estoque_snapshot WHERE codigo_estoque LIKE 'ETL-%'",
        "DELETE FROM pcp.execucao_pipeline WHERE data_ref = DATE '2099-07-15'",
    ] {
        sqlx::query(sql).execute(pool).await.expect("limpeza");
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
async fn importa_csv_e_processa_o_motor() {
    let (pool, config) = preparar().await;

    let dir = std::env::temp_dir();
    let caminho_vendas = dir.join("pcp_etl_teste_vendas.csv");
    let caminho_snapshot = dir.join("pcp_etl_teste_snapshot.csv");
    std::fs::write(&caminho_vendas, VENDAS).expect("escrever vendas.csv");
    std::fs::write(&caminho_snapshot, SNAPSHOT).expect("escrever snapshot.csv");

    // Importação idempotente pela FonteDados de arquivo.
    let fonte = ImportadorArquivo::novo(&caminho_vendas, &caminho_snapshot);
    let resumo = importar(&pool, &fonte).await.expect("importar");
    assert_eq!(resumo.linhas_vendas, 2);
    assert_eq!(resumo.linhas_snapshot, 1);
    assert!(
        conta(
            &pool,
            "SELECT COUNT(*) FROM pcp.vendas_dia WHERE codigo_estoque = 'ETL-A'"
        )
        .await
            >= 2
    );

    // Reimportar é idempotente (substitui o dia, não acumula).
    importar(&pool, &fonte).await.expect("reimportar");
    assert_eq!(
        conta(
            &pool,
            "SELECT COUNT(*) FROM pcp.vendas_dia WHERE codigo_estoque = 'ETL-A'"
        )
        .await,
        2
    );

    // O motor roda sobre os dados importados.
    let res = processar_dia(
        &pool,
        &config,
        NaiveDate::from_ymd_opt(2099, 7, 15).unwrap(),
    )
    .await
    .expect("processar");
    assert_eq!(res.status, StatusPipeline::Completo);
    assert_eq!(
        conta(&pool, "SELECT COUNT(*) FROM pcp.classificacao WHERE codigo_estoque = 'ETL-A' AND dt_calculo = DATE '2099-07-15'").await,
        1
    );

    limpar(&pool).await;
    let _ = std::fs::remove_file(&caminho_vendas);
    let _ = std::fs::remove_file(&caminho_snapshot);
}
