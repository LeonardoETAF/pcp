//! Teste de integração da sazonalidade (precisa de Postgres de teste — `TEST_DATABASE_URL`, nunca
//! o de desenvolvimento). `#[ignore]`. Rode com: `TEST_DATABASE_URL=... cargo test -p pcp-engine -- --ignored`.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

use chrono::NaiveDate;

use pcp_core::sazonalidade::ParametrosSazonalidade;
use pcp_db::{sazonalidade as db, vendas, NovaVendaDia, PgPool};
use pcp_engine::sazonalidade::{atualizar_fatores, ResultadoSazonalidade};

async fn pool() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("defina TEST_DATABASE_URL (banco de teste dedicado — nunca o de desenvolvimento)");
    let pool = pcp_db::criar_pool(&url, 5).await.expect("pool");
    pcp_db::aplicar_migrations(&pool).await.expect("migrations");
    pool
}

fn params() -> ParametrosSazonalidade {
    ParametrosSazonalidade {
        clamp_min: 0.5,
        clamp_max: 2.0,
        atualizar_apos_dias: 30,
    }
}

fn venda(dia: NaiveDate, qtd: i64) -> NovaVendaDia {
    NovaVendaDia {
        dt_ref: dia,
        codigo_estoque: "SAZ-TESTE".to_owned(),
        sku: None,
        produto: Some("Copo".to_owned()),
        configuracao: None,
        qtd_vendida: i32::try_from(qtd).unwrap_or(0),
        is_personalizado: false,
    }
}

#[tokio::test]
#[ignore = "precisa de Postgres de teste (TEST_DATABASE_URL); rode com --ignored"]
async fn recalcula_persiste_e_respeita_o_gatilho() {
    let pool = pool().await;

    // Limpa a janela do ano anterior (2025) para o produto de teste e os fatores.
    for mes in 1..=12 {
        let dia = NaiveDate::from_ymd_opt(2025, mes, 1).unwrap();
        vendas::substituir_dia(&pool, dia, &[]).await.unwrap();
    }
    sqlx_truncate_fatores(&pool).await;

    // Dezembro vende muito mais que os demais meses -> fator de dezembro deve ser o maior.
    vendas::substituir_dia(
        &pool,
        NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(),
        &[venda(NaiveDate::from_ymd_opt(2025, 6, 1).unwrap(), 10)],
    )
    .await
    .unwrap();
    vendas::substituir_dia(
        &pool,
        NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(),
        &[venda(NaiveDate::from_ymd_opt(2025, 12, 1).unwrap(), 100)],
    )
    .await
    .unwrap();

    let hoje = NaiveDate::from_ymd_opt(2026, 6, 15).unwrap();

    // 1) Gatilho dispara (tabela vazia) -> recalcula e persiste os 12 fatores.
    assert_eq!(
        atualizar_fatores(&pool, hoje, params()).await.unwrap(),
        ResultadoSazonalidade::Recalculado
    );
    let fatores = db::listar(&pool).await.unwrap();
    assert_eq!(fatores.len(), 12);

    let fator_de = |mes: i16| {
        fatores
            .iter()
            .find(|(m, _)| *m == mes)
            .map(|(_, f)| *f)
            .unwrap()
    };
    // Dezembro (alto) > junho (baixo); ambos dentro do clamp [0.5, 2.0].
    assert!(fator_de(12) > fator_de(6));
    assert!(fator_de(12) <= 2.0 && fator_de(6) >= 0.5);

    // 2) Logo após recalcular, no mesmo mês -> gatilho não dispara.
    assert_eq!(
        atualizar_fatores(&pool, hoje, params()).await.unwrap(),
        ResultadoSazonalidade::NaoNecessario
    );

    // Limpa a data de teste.
    for mes in [6, 12] {
        let dia = NaiveDate::from_ymd_opt(2025, mes, 1).unwrap();
        vendas::substituir_dia(&pool, dia, &[]).await.unwrap();
    }
}

async fn sqlx_truncate_fatores(pool: &PgPool) {
    sqlx::query("TRUNCATE pcp.fatores_sazonais")
        .execute(pool)
        .await
        .expect("truncate fatores");
}
