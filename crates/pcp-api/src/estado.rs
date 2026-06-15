//! Estado compartilhado da aplicação e configuração lida do ambiente (CLAUDE.md §7.4).

use std::net::SocketAddr;
use std::sync::Arc;

use chrono::Duration;
use pcp_db::PgPool;
use tokio::sync::broadcast;

/// Capacidade do canal de eventos em tempo real (SSE — §16); receptores lentos só perdem eventos.
const CAPACIDADE_EVENTOS: usize = 64;

/// Estado compartilhado entre handlers (clonável — `PgPool`/`Arc`/`Sender` são baratos).
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub jwt_secret: Arc<Vec<u8>>,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
    eventos: broadcast::Sender<String>,
}

impl AppState {
    /// Monta o estado a partir do pool e dos parâmetros de token.
    #[must_use]
    pub fn novo(
        pool: PgPool,
        jwt_secret: Vec<u8>,
        access_ttl: Duration,
        refresh_ttl: Duration,
    ) -> Self {
        let (eventos, _) = broadcast::channel(CAPACIDADE_EVENTOS);
        Self {
            pool,
            jwt_secret: Arc::new(jwt_secret),
            access_ttl,
            refresh_ttl,
            eventos,
        }
    }

    /// Emissor para publicar eventos de tempo real (usado pela ponte LISTEN/NOTIFY → SSE).
    #[must_use]
    pub fn emissor(&self) -> broadcast::Sender<String> {
        self.eventos.clone()
    }

    /// Inscreve um novo assinante no fluxo de eventos (usado pelo endpoint SSE).
    #[must_use]
    pub fn assinar(&self) -> broadcast::Receiver<String> {
        self.eventos.subscribe()
    }
}

/// Erros de bootstrap (leitura do ambiente e preparação do servidor).
#[derive(Debug, thiserror::Error)]
pub enum ErroBootstrap {
    /// Variável de ambiente obrigatória ausente.
    #[error("variável de ambiente obrigatória ausente: {0}")]
    VarAusente(String),
    /// Valor presente porém inválido.
    #[error("valor inválido para {chave}: {motivo}")]
    ValorInvalido { chave: String, motivo: String },
    /// Falha ao gerar hash de senha no bootstrap do admin.
    #[error("falha ao gerar hash de senha")]
    Hashing,
    /// Falha de banco.
    #[error(transparent)]
    Db(#[from] pcp_db::ErroDb),
}

/// Configuração do servidor, lida das variáveis de ambiente.
pub struct ConfigApi {
    pub database_url: String,
    pub jwt_secret: Vec<u8>,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
    pub bind_addr: SocketAddr,
    pub admin_email: Option<String>,
    pub admin_senha: Option<String>,
}

impl ConfigApi {
    /// Lê a configuração do ambiente. Secrets nunca são hardcoded (CLAUDE.md §7.4).
    ///
    /// # Errors
    /// [`ErroBootstrap::VarAusente`] / [`ErroBootstrap::ValorInvalido`] conforme o ambiente.
    pub fn do_ambiente() -> Result<Self, ErroBootstrap> {
        let database_url = obrigatoria("DATABASE_URL")?;
        let jwt_secret = obrigatoria("PCP_JWT_SECRET")?;
        if jwt_secret.len() < 32 {
            return Err(ErroBootstrap::ValorInvalido {
                chave: "PCP_JWT_SECRET".into(),
                motivo: "use ao menos 32 caracteres".into(),
            });
        }
        let bind = std::env::var("PCP_API_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());
        let bind_addr =
            bind.parse()
                .map_err(|e: std::net::AddrParseError| ErroBootstrap::ValorInvalido {
                    chave: "PCP_API_BIND".into(),
                    motivo: e.to_string(),
                })?;
        Ok(Self {
            database_url,
            jwt_secret: jwt_secret.into_bytes(),
            access_ttl: Duration::minutes(inteiro_opcional("PCP_ACCESS_TTL_MIN", 15)?),
            refresh_ttl: Duration::days(inteiro_opcional("PCP_REFRESH_TTL_DIAS", 7)?),
            bind_addr,
            admin_email: std::env::var("PCP_ADMIN_EMAIL").ok(),
            admin_senha: std::env::var("PCP_ADMIN_SENHA").ok(),
        })
    }
}

fn obrigatoria(chave: &str) -> Result<String, ErroBootstrap> {
    std::env::var(chave).map_err(|_| ErroBootstrap::VarAusente(chave.to_owned()))
}

fn inteiro_opcional(chave: &str, padrao: i64) -> Result<i64, ErroBootstrap> {
    match std::env::var(chave) {
        Ok(v) => v.parse().map_err(|_| ErroBootstrap::ValorInvalido {
            chave: chave.to_owned(),
            motivo: "esperado um inteiro".into(),
        }),
        Err(_) => Ok(padrao),
    }
}
