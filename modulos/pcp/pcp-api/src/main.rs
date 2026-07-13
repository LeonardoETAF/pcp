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
    let yaml = pcp_config::carregar_de_arquivo(&cfg.config_path)?;
    let pool = pcp_db::criar_pool(&cfg.database_url, 10).await?;
    // ORDEM IMPORTA: o módulo primeiro, o núcleo depois.
    // A migration 0004 do PCP ainda é quem CRIA `pcp.usuario` (histórico já aplicado em
    // produção — não dá para reescrevê-la sem quebrar o checksum). A migration do núcleo
    // então MOVE essa tabela para `nucleo.*`. Rodar o núcleo antes, num banco novo, não
    // encontraria nada para mover. Ver nucleo/migrations/0001_identidade.sql.
    pcp_db::aplicar_migrations(&pool).await?;
    sf_db::aplicar_migrations_nucleo(&pool).await?;
    // Config efetiva: a persistida no banco (se houver) tem prioridade sobre o YAML default.
    let config = std::sync::Arc::new(config_efetiva(&pool, yaml).await);

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

/// Config efetiva no startup: usa a persistida no banco se existir e for válida; senão o YAML.
async fn config_efetiva(pool: &pcp_db::PgPool, yaml: pcp_config::Config) -> pcp_config::Config {
    match pcp_db::config_persist::carregar(pool).await {
        Ok(Some(valor)) => match serde_json::from_value::<pcp_config::Config>(valor) {
            Ok(c) if pcp_config::validar(&c).is_ok() => c,
            _ => {
                tracing::warn!("config persistida inválida; usando o YAML default");
                yaml
            }
        },
        Ok(None) => yaml,
        Err(e) => {
            tracing::warn!(%e, "falha ao ler config persistida; usando o YAML default");
            yaml
        }
    }
}
