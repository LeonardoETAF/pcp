//! Erros tipados do motor (CLAUDE.md §5).

use thiserror::Error;

/// Falhas da orquestração do motor.
#[derive(Debug, Error)]
pub enum ErroEngine {
    /// Falha de acesso a dados.
    #[error(transparent)]
    Db(#[from] pcp_db::ErroDb),

    /// Data inválida na montagem de um intervalo (ex.: ano anterior).
    #[error("data inválida no cálculo")]
    DataInvalida,
}
