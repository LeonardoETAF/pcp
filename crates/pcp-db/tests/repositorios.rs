//! Testes de integração dos repositórios — exigem um Postgres real (`DATABASE_URL`).
//! Marcados `#[ignore]` para não quebrar o CI sem banco. Rode localmente com:
//!   `docker compose up -d` e depois `cargo test -p pcp-db -- --ignored`.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

use chrono::NaiveDate;
use pcp_db::{aplicar_migrations, criar_pool, snapshot, vendas, NovaVendaDia, NovoEstoqueSnapshot};
use sqlx::PgPool;

async fn pool_de_teste() -> PgPool {
    let url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL deve estar definida para os testes de banco");
    let pool = criar_pool(&url, 5).await.expect("conexão com o Postgres");
    aplicar_migrations(&pool).await.expect("aplicar migrations");
    pool
}

fn data_teste() -> NaiveDate {
    NaiveDate::from_ymd_opt(2099, 1, 1).expect("data válida")
}

fn venda(codigo: &str, qtd: i32, personalizado: bool) -> NovaVendaDia {
    NovaVendaDia {
        dt_ref: data_teste(),
        codigo_estoque: codigo.to_owned(),
        sku: Some(format!("SKU-{codigo}")),
        produto: Some("Copo".to_owned()),
        configuracao: Some("COR DO PRODUTO: AZUL".to_owned()),
        qtd_vendida: qtd,
        is_personalizado: personalizado,
    }
}

#[tokio::test]
#[ignore = "precisa de Postgres (DATABASE_URL); rode com --ignored"]
async fn vendas_substituir_dia_e_idempotencia() {
    let pool = pool_de_teste().await;
    let d = data_teste();

    // Várias linhas por (dt_ref, codigo) — variações LISO/PERSONALIZADO (doc 02 §1).
    let lote = [
        venda("6797", 10, false),
        venda("6797", 3, true),
        venda("10001", 7, false),
    ];
    let inseridas = vendas::substituir_dia(&pool, d, &lote).await.unwrap();
    assert_eq!(inseridas, 3);
    assert_eq!(vendas::do_dia(&pool, d).await.unwrap().len(), 3);
    assert_eq!(vendas::contar_do_dia(&pool, d).await.unwrap(), 3);

    // Reprocessar a mesma data substitui sem acumular (idempotência — CLAUDE.md §3.3).
    let inseridas2 = vendas::substituir_dia(&pool, d, &[venda("6797", 5, false)])
        .await
        .unwrap();
    assert_eq!(inseridas2, 1);
    assert_eq!(vendas::contar_do_dia(&pool, d).await.unwrap(), 1);

    // Limpa a data de teste.
    vendas::substituir_dia(&pool, d, &[]).await.unwrap();
    assert_eq!(vendas::contar_do_dia(&pool, d).await.unwrap(), 0);
}

fn snap(codigo: &str, estoque: i32, reserva: i32, fora: bool) -> NovoEstoqueSnapshot {
    NovoEstoqueSnapshot {
        dt_ref: data_teste(),
        codigo_estoque: codigo.to_owned(),
        sku: None,
        produto: Some("Copo".to_owned()),
        configuracao: None,
        qtd_estoque: estoque,
        qtd_reserva: reserva,
        qtd_disponivel: estoque - reserva,
        estoque_min_erp: Some(100),
        fora_de_linha: fora,
    }
}

#[tokio::test]
#[ignore = "precisa de Postgres (DATABASE_URL); rode com --ignored"]
async fn snapshot_substituir_dia_e_mais_recente() {
    let pool = pool_de_teste().await;
    let d = data_teste();

    let inseridos = snapshot::substituir_dia(
        &pool,
        d,
        &[snap("6797", 500, 50, false), snap("10001", 0, 0, true)],
    )
    .await
    .unwrap();
    assert_eq!(inseridos, 2);

    let linhas = snapshot::do_dia(&pool, d).await.unwrap();
    assert_eq!(linhas.len(), 2);
    let s6797 = linhas
        .iter()
        .find(|s| s.codigo_estoque == "6797")
        .expect("6797 presente");
    assert_eq!(s6797.qtd_disponivel, 450); // 500 - 50

    // Snapshot completo do dia: re-substituir troca tudo daquela data.
    let inseridos2 = snapshot::substituir_dia(&pool, d, &[snap("6797", 600, 100, false)])
        .await
        .unwrap();
    assert_eq!(inseridos2, 1);
    assert_eq!(snapshot::do_dia(&pool, d).await.unwrap().len(), 1);

    // Há um snapshot ao menos tão recente quanto o inserido (outros testes podem ter datas
    // futuras próprias; a comparação é robusta a isso).
    assert!(snapshot::data_mais_recente(&pool).await.unwrap() >= Some(d));

    // Limpa a data de teste.
    snapshot::substituir_dia(&pool, d, &[]).await.unwrap();
}
