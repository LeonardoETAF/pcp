//! Teste de paridade com o legado (doc 08 §3–4). Precisa de Postgres E de um dump real do
//! legado. `#[ignore]`. Pula com aviso se o dump não for fornecido.
//!
//! Como rodar (com o dump real):
//! ```sh
//! PCP_DUMP_VENDAS=/caminho/vendas.csv PCP_DUMP_SNAPSHOT=/caminho/snapshot.csv \
//!   PCP_DUMP_DATA_REF=2026-06-15 cargo test -p pcp-tests --test paridade -- --ignored
//! ```
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

use std::path::PathBuf;

use chrono::NaiveDate;

use pcp_db::PgPool;
use pcp_engine::{processar_dia, StatusPipeline};
use pcp_etl::{importar, ImportadorArquivo};

/// Tolerância de contagem por classe (doc 08 §3: "teste de aceitação aproximado").
const TOLERANCIA: i64 = 5;

async fn conta_classe(pool: &PgPool, data_ref: NaiveDate, classe: char) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pcp.classificacao WHERE dt_calculo = $1 AND classe = $2",
    )
    .bind(data_ref)
    .bind(classe.to_string())
    .fetch_one(pool)
    .await
    .expect("contagem por classe")
}

async fn esta_classificado(pool: &PgPool, data_ref: NaiveDate, codigo: &str) -> bool {
    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM pcp.classificacao WHERE dt_calculo = $1 AND codigo_estoque = $2",
    )
    .bind(data_ref)
    .bind(codigo)
    .fetch_one(pool)
    .await
    .expect("contagem do produto");
    total > 0
}

#[tokio::test]
#[ignore = "precisa de Postgres + dump real (PCP_DUMP_VENDAS/PCP_DUMP_SNAPSHOT)"]
async fn paridade_com_dump_do_legado() {
    let (Ok(vendas), Ok(snapshot)) = (
        std::env::var("PCP_DUMP_VENDAS"),
        std::env::var("PCP_DUMP_SNAPSHOT"),
    ) else {
        eprintln!(
            "PARIDADE PULADA: defina PCP_DUMP_VENDAS e PCP_DUMP_SNAPSHOT com os CSV do dump real."
        );
        return;
    };
    let data_ref: NaiveDate = std::env::var("PCP_DUMP_DATA_REF")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| NaiveDate::from_ymd_opt(2026, 6, 15).expect("data padrão"));

    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL");
    let pool = pcp_db::criar_pool(&url, 5).await.expect("pool");
    pcp_db::aplicar_migrations(&pool).await.expect("migrations");
    let config = pcp_config::carregar_de_arquivo(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../config/pcp.config.yaml"),
    )
    .expect("config");

    // Importa o dump real e roda o motor sobre a data de referência.
    let fonte = ImportadorArquivo::novo(vendas, snapshot);
    importar(&pool, &fonte).await.expect("importar dump");
    let res = processar_dia(&pool, &config, data_ref)
        .await
        .expect("processar");
    assert_eq!(res.status, StatusPipeline::Completo);

    // Distribuição esperada (doc 08 §3) — com tolerância de arredondamento.
    for (classe, esperado) in [
        ('A', 165),
        ('B', 346),
        ('C', 671),
        ('D', 1012),
        ('F', 177),
        ('N', 9),
    ] {
        let obtido = conta_classe(&pool, data_ref, classe).await;
        assert!(
            (obtido - esperado).abs() <= TOLERANCIA,
            "classe {classe}: obtido {obtido}, esperado ~{esperado}"
        );
    }

    // Produtos de referência classificados (doc 08 §4).
    for codigo in ["6797", "10001", "10473"] {
        assert!(
            esta_classificado(&pool, data_ref, codigo).await,
            "produto de referência {codigo} não foi classificado"
        );
    }
}
