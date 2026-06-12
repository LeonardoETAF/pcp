//! Erros tipados do carregamento/validação da configuração (CLAUDE.md §5).

use thiserror::Error;

/// Falhas ao carregar ou validar `config/pcp.config.yaml`.
#[derive(Debug, Error)]
pub enum ErroConfig {
    /// O arquivo não pôde ser lido do disco.
    #[error("falha ao ler a configuração '{caminho}': {origem}")]
    Leitura {
        /// Caminho que se tentou ler.
        caminho: String,
        /// Erro de I/O subjacente.
        #[source]
        origem: std::io::Error,
    },

    /// O conteúdo não é um YAML válido para o esquema esperado.
    #[error("YAML de configuração inválido: {0}")]
    Yaml(#[from] serde_norway::Error),

    /// O YAML casa com o esquema, mas viola invariantes de negócio.
    #[error("configuração inválida:\n- {}", .0.join("\n- "))]
    Validacao(Vec<String>),
}
