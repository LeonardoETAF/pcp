//! Erro de API mapeado para resposta HTTP. Mensagens genéricas — não vazam detalhe interno.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

/// Erros expostos pela API.
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Falta autenticação ou o token é inválido/expirado.
    #[error("não autenticado")]
    NaoAutenticado,
    /// Autenticado, mas sem privilégio suficiente.
    #[error("acesso negado")]
    Proibido,
    /// E-mail/senha inválidos no login.
    #[error("credenciais inválidas")]
    CredenciaisInvalidas,
    /// Requisição malformada/dados inválidos.
    #[error("{0}")]
    Requisicao(String),
    /// Conflito de estado (ex.: e-mail já cadastrado).
    #[error("{0}")]
    Conflito(String),
    /// Recurso inexistente (ex.: produto não encontrado).
    #[error("não encontrado")]
    NaoEncontrado,
    /// Falha interna (detalhe registrado no log, não exposto).
    #[error("erro interno")]
    Interno,
}

impl ApiError {
    fn status(&self) -> StatusCode {
        match self {
            ApiError::NaoAutenticado | ApiError::CredenciaisInvalidas => StatusCode::UNAUTHORIZED,
            ApiError::Proibido => StatusCode::FORBIDDEN,
            ApiError::Requisicao(_) => StatusCode::BAD_REQUEST,
            ApiError::Conflito(_) => StatusCode::CONFLICT,
            ApiError::NaoEncontrado => StatusCode::NOT_FOUND,
            ApiError::Interno => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        (status, Json(json!({ "erro": self.to_string() }))).into_response()
    }
}

impl From<pcp_db::ErroDb> for ApiError {
    fn from(erro: pcp_db::ErroDb) -> Self {
        tracing::error!(%erro, "erro de banco");
        ApiError::Interno
    }
}
