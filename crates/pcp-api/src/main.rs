//! Binário do servidor `pcp-api`. Bootstrap: ambiente → pool → migrations → admin → serve.
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::pedantic)]

use pcp_api::{bootstrap, rotas, AppState, ConfigApi};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cfg = ConfigApi::do_ambiente()?;
    let config = std::sync::Arc::new(pcp_config::carregar_de_arquivo(&cfg.config_path)?);
    let pool = pcp_db::criar_pool(&cfg.database_url, 10).await?;
    pcp_db::aplicar_migrations(&pool).await?;

    if let (Some(email), Some(senha)) = (&cfg.admin_email, &cfg.admin_senha) {
        bootstrap::garantir_admin_inicial(&pool, email, senha).await?;
    }

    let estado = AppState::novo(
        pool,
        cfg.jwt_secret,
        cfg.access_ttl,
        cfg.refresh_ttl,
        config,
    );

    // Ponte de tempo real (SSE — §16): escuta o pipeline (LISTEN/NOTIFY) numa task dedicada.
    tokio::spawn(pcp_api::escutar_pipeline(
        cfg.database_url.clone(),
        estado.emissor(),
    ));

    let app = rotas(estado);

    let listener = tokio::net::TcpListener::bind(cfg.bind_addr).await?;
    tracing::info!("pcp-api ouvindo em http://{}", cfg.bind_addr);
    axum::serve(listener, app).await?;
    Ok(())
}
