//! Erros tipados do ETL (CLAUDE.md §5).

use thiserror::Error;

/// Falhas na leitura, validação ou gravação dos dados de entrada.
#[derive(Debug, Error)]
pub enum ErroEtl {
    /// Falha ao abrir o arquivo.
    #[error("falha ao abrir '{caminho}': {origem}")]
    Io {
        caminho: String,
        #[source]
        origem: std::io::Error,
    },
    /// Erro de parsing do CSV.
    #[error("erro ao ler CSV: {0}")]
    Csv(#[from] csv::Error),
    /// Violação do contrato de dados (doc 05 §2), com a linha do arquivo.
    #[error("linha {linha}: {motivo}")]
    Validacao { linha: usize, motivo: String },
    /// Falha ao gravar no banco.
    #[error(transparent)]
    Db(#[from] pcp_db::ErroDb),
    /// Falha na consulta somente-leitura ao ERP One (camada anticorrupção, §1/§8).
    #[error("falha na consulta ao One: {0}")]
    One(#[from] sqlx::Error),
}
