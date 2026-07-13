//! Erros tipados do acesso a dados, compartilhados por todos os módulos (CLAUDE.md §5).

use thiserror::Error;

/// Falhas das operações de banco.
#[derive(Debug, Error)]
pub enum ErroDb {
    /// Erro vindo do driver/consulta `SQLx`.
    #[error("erro de banco: {0}")]
    Sqlx(#[from] sqlx::Error),

    /// Erro ao aplicar migrations versionadas.
    #[error("erro ao aplicar migrations: {0}")]
    Migracao(#[from] sqlx::migrate::MigrateError),
}
