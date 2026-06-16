//! Testes de contrato dos endpoints de leitura (precisam de Postgres — `DATABASE_URL`).
//! `#[ignore]`. Semeiam `produto_ativo`/`alerta`, conferem o formato do payload, a paginação no
//! servidor, a exclusão da cobertura 999 das médias (§11) e o deny-by-default (§7).
//! Rode com: `docker compose up -d` e `cargo test -p pcp-api --test leitura -- --ignored`.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use serde_json::Value;
use tower::ServiceExt; // .oneshot
use uuid::Uuid;

use pcp_api::{rotas, senha, AppState};
use pcp_db::usuarios;

const SEGREDO: &[u8] = b"segredo-de-teste-com-mais-de-32-bytes!!";

async fn estado_de_teste() -> AppState {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL para os testes de banco");
    let pool = pcp_db::criar_pool(&url, 5).await.expect("pool");
    pcp_db::aplicar_migrations(&pool).await.expect("migrations");
    let config = std::sync::Arc::new(
        pcp_config::carregar_de_arquivo(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../config/pcp.config.yaml"
        ))
        .expect("config de negócio"),
    );
    AppState::novo(
        pool,
        SEGREDO.to_vec(),
        Duration::minutes(15),
        Duration::days(7),
        config,
    )
}

/// Reseta a "view" materializada e semeia dois produtos + um alerta para o dia.
/// LEIT-A: classe A, crítico, sugere 100, cobertura 5.0. LEIT-B: classe C, cobertura 999
/// (sem histórico) — deve ficar FORA da média (§11).
async fn semear(estado: &AppState) {
    let pool = &estado.pool;
    sqlx::query("TRUNCATE pcp.produto_ativo")
        .execute(pool)
        .await
        .unwrap();
    let hoje = Utc::now().date_naive();
    for (codigo, classe, status, sugerida, cobertura, media, volume) in [
        (
            "LEIT-A", "A", "critico", 100_i64, 5.0_f64, 10.0_f64, 5_000_i64,
        ),
        ("LEIT-B", "C", "sem_historico", 0, 999.0, 0.0, 0),
    ] {
        sqlx::query(
            "INSERT INTO pcp.produto_ativo \
             (codigo_estoque, sku, produto, configuracao, classe, fator_estoque, qtd_estoque, \
              qtd_reserva, qtd_disponivel, media_diaria, coef_variacao, dias_com_vendas, \
              estoque_minimo, estoque_seguranca, estoque_total_recomendado, cobertura_dias, \
              status, qtd_sugerida, fora_de_linha, volume_janela, dt_ref) \
             VALUES ($1,$2,$3,$4,$5,1.0,100,10,90,$6,0.1,12,50,30,$7,$8,$9,$10,false,$11,$12)",
        )
        .bind(codigo)
        .bind(format!("SKU-{codigo}"))
        .bind("Copo")
        .bind("COR DO PRODUTO: AZUL")
        .bind(classe)
        .bind(media)
        .bind(200_i64)
        .bind(cobertura)
        .bind(status)
        .bind(sugerida)
        .bind(volume)
        .bind(hoje)
        .execute(pool)
        .await
        .unwrap();
    }
    sqlx::query("DELETE FROM pcp.alerta WHERE codigo_estoque LIKE 'LEIT-%'")
        .execute(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO pcp.alerta (dt_alerta, codigo_estoque, prioridade, classe, qtd_sugerida, cobertura_dias) \
         VALUES ($1, 'LEIT-A', 'critico', 'A', 100, 5.0)",
    )
    .bind(hoje)
    .execute(pool)
    .await
    .unwrap();
}

async fn token_analista(estado: &AppState) -> String {
    let email = format!("leit-{}@teste.local", Uuid::new_v4());
    let hash = senha::hashear("senha-de-teste-123").unwrap();
    usuarios::criar(&estado.pool, &email, &hash, "analista", Some("Teste"))
        .await
        .unwrap();
    let corpo = serde_json::json!({ "email": email, "senha": "senha-de-teste-123" }).to_string();
    let resp = rotas(estado.clone())
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/auth/login")
                .header("content-type", "application/json")
                .body(Body::from(corpo))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: Value = serde_json::from_slice(&bytes).unwrap();
    v["access_token"].as_str().unwrap().to_owned()
}

async fn get_json(estado: &AppState, uri: &str, token: &str) -> (StatusCode, Value) {
    let resp = rotas(estado.clone())
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(uri)
                .header("authorization", format!("Bearer {token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}

#[tokio::test]
#[ignore = "precisa de Postgres (DATABASE_URL); rode com --ignored"]
async fn endpoints_de_leitura_entregam_o_contrato() {
    let estado = estado_de_teste().await;
    semear(&estado).await;
    let token = token_analista(&estado).await;

    // Dashboard: total 2, soma de sugeridas 100, cobertura média = 5.0 (999 excluído — §11).
    let (st, dash) = get_json(&estado, "/pcp/dashboard", &token).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(dash["total_produtos"], 2);
    assert_eq!(dash["total_sugerido"], 100);
    assert!((dash["cobertura_media"].as_f64().unwrap() - 5.0).abs() < 1e-9);

    // Estoque paginado: filtro por classe A devolve só LEIT-A; total reflete o filtro.
    let (st, pag) = get_json(&estado, "/pcp/estoque?classe=A&limite=10", &token).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(pag["total"], 1);
    assert_eq!(pag["limite"], 10);
    let itens = pag["itens"].as_array().unwrap();
    assert_eq!(itens.len(), 1);
    assert_eq!(itens[0]["codigo_estoque"], "LEIT-A");
    assert_eq!(itens[0]["status"], "critico");
    assert_eq!(itens[0]["qtd_sugerida"], 100);

    // Alertas do dia: 1 alerta, enriquecido com dados do produto.
    let (st, alertas) = get_json(&estado, "/pcp/alertas", &token).await;
    assert_eq!(st, StatusCode::OK);
    let lista = alertas.as_array().unwrap();
    assert_eq!(lista.len(), 1);
    assert_eq!(lista[0]["codigo_estoque"], "LEIT-A");
    assert_eq!(lista[0]["prioridade"], "critico");
    assert_eq!(lista[0]["configuracao"], "COR DO PRODUTO: AZUL");

    // ABC: distribuição cobre as classes semeadas (A e C).
    let (st, abc) = get_json(&estado, "/pcp/abc", &token).await;
    assert_eq!(st, StatusCode::OK);
    let dist = abc.as_array().unwrap();
    assert!(dist
        .iter()
        .any(|d| d["classe"] == "A" && d["quantidade"] == 1));
    assert!(dist
        .iter()
        .any(|d| d["classe"] == "C" && d["quantidade"] == 1));

    // Limpeza.
    sqlx::query("DELETE FROM pcp.alerta WHERE codigo_estoque LIKE 'LEIT-%'")
        .execute(&estado.pool)
        .await
        .unwrap();
    sqlx::query("TRUNCATE pcp.produto_ativo")
        .execute(&estado.pool)
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "precisa de Postgres (DATABASE_URL); rode com --ignored"]
async fn leitura_exige_autenticacao() {
    let estado = estado_de_teste().await;
    for uri in ["/pcp/dashboard", "/pcp/estoque", "/pcp/alertas", "/pcp/abc"] {
        let resp = rotas(estado.clone())
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED, "{uri} sem token");
    }
}
