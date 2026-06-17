//! Testes de autorização ponta a ponta (precisam de um Postgres de teste — `TEST_DATABASE_URL`,
//! NUNCA o banco de desenvolvimento). `#[ignore]`. Rode com:
//! `TEST_DATABASE_URL=postgres://pcp:...@localhost:5433/pcp_test cargo test -p pcp-api -- --ignored`.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

use axum::body::Body;
use axum::http::{Request, StatusCode};
use chrono::Duration;
use http_body_util::BodyExt;
use tower::ServiceExt; // .oneshot
use uuid::Uuid;

use pcp_api::{rotas, senha, AppState};
use pcp_db::usuarios;

const SEGREDO: &[u8] = b"segredo-de-teste-com-mais-de-32-bytes!!";

async fn estado_de_teste() -> AppState {
    let url = std::env::var("TEST_DATABASE_URL")
        .expect("defina TEST_DATABASE_URL (banco de teste dedicado — nunca o de desenvolvimento)");
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

async fn criar_usuario_teste(estado: &AppState, papel: &str) -> String {
    let email = format!("{papel}-{}@teste.local", Uuid::new_v4());
    let hash = senha::hashear("senha-de-teste-123").unwrap();
    usuarios::criar(&estado.pool, &email, &hash, papel, Some("Teste"))
        .await
        .unwrap();
    email
}

/// Apaga um usuário de teste pelo e-mail (CASCADE limpa tokens/filtros/preferências). Como a
/// autorização é stateless (valida só o JWT), o token continua válido após a remoção — limpeza
/// sem deixar resíduo no banco e segura entre testes paralelos (cada um remove só o que criou).
async fn apagar_usuario(estado: &AppState, email: &str) {
    sqlx::query("DELETE FROM pcp.usuario WHERE email = $1")
        .bind(email)
        .execute(&estado.pool)
        .await
        .expect("limpar usuário de teste");
}

/// Cria um usuário do papel, faz login (precisa do usuário no banco) e JÁ o remove, devolvendo o
/// token — que sobrevive por ser stateless. Não fica usuário de teste no banco.
async fn token_de(estado: &AppState, papel: &str) -> String {
    let email = criar_usuario_teste(estado, papel).await;
    let token = logar(estado, &email).await;
    apagar_usuario(estado, &email).await;
    token
}

async fn logar(estado: &AppState, email: &str) -> String {
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
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    v["access_token"].as_str().unwrap().to_owned()
}

async fn status_get(estado: &AppState, uri: &str, token: Option<&str>) -> StatusCode {
    let mut req = Request::builder().method("GET").uri(uri);
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    rotas(estado.clone())
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap()
        .status()
}

async fn status_post_usuario(estado: &AppState, token: Option<&str>, email: &str) -> StatusCode {
    let corpo = serde_json::json!({
        "email": email,
        "senha": "senha-de-teste-123",
        "papel": "analista"
    })
    .to_string();
    let mut req = Request::builder()
        .method("POST")
        .uri("/pcp/usuarios")
        .header("content-type", "application/json");
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    rotas(estado.clone())
        .oneshot(req.body(Body::from(corpo)).unwrap())
        .await
        .unwrap()
        .status()
}

#[tokio::test]
#[ignore = "precisa de Postgres de teste (TEST_DATABASE_URL); rode com --ignored"]
async fn anonimo_nao_acessa_nada_em_pcp() {
    let estado = estado_de_teste().await;
    assert_eq!(
        status_get(&estado, "/pcp/me", None).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status_get(&estado, "/pcp/aprovacoes", None).await,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        status_post_usuario(&estado, None, "anon@teste.local").await,
        StatusCode::UNAUTHORIZED
    );
    // token inválido também é barrado
    assert_eq!(
        status_get(&estado, "/pcp/me", Some("lixo.invalido.xyz")).await,
        StatusCode::UNAUTHORIZED
    );
    // rota pública continua acessível
    assert_eq!(status_get(&estado, "/saude", None).await, StatusCode::OK);
}

#[tokio::test]
#[ignore = "precisa de Postgres de teste (TEST_DATABASE_URL); rode com --ignored"]
async fn cada_papel_acessa_o_que_deve() {
    let estado = estado_de_teste().await;
    let analista = token_de(&estado, "analista").await;
    let gestor = token_de(&estado, "gestor").await;
    let admin = token_de(&estado, "admin").await;

    // /pcp/me: qualquer autenticado
    assert_eq!(
        status_get(&estado, "/pcp/me", Some(&analista)).await,
        StatusCode::OK
    );
    assert_eq!(
        status_get(&estado, "/pcp/me", Some(&gestor)).await,
        StatusCode::OK
    );
    assert_eq!(
        status_get(&estado, "/pcp/me", Some(&admin)).await,
        StatusCode::OK
    );

    // /pcp/aprovacoes: gestor ou admin
    assert_eq!(
        status_get(&estado, "/pcp/aprovacoes", Some(&analista)).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status_get(&estado, "/pcp/aprovacoes", Some(&gestor)).await,
        StatusCode::OK
    );
    assert_eq!(
        status_get(&estado, "/pcp/aprovacoes", Some(&admin)).await,
        StatusCode::OK
    );

    // POST /pcp/usuarios: somente admin. Reusa o mesmo e-mail: só o admin (CREATED) o cria.
    let novo = format!("novo-{}@teste.local", Uuid::new_v4());
    assert_eq!(
        status_post_usuario(&estado, Some(&analista), &novo).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status_post_usuario(&estado, Some(&gestor), &novo).await,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        status_post_usuario(&estado, Some(&admin), &novo).await,
        StatusCode::CREATED
    );
    apagar_usuario(&estado, &novo).await; // remove o usuário criado pela API
}
