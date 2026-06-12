//! Carregamento da configuração a partir de arquivo ou string YAML (doc 02 §11).

use std::path::Path;

use crate::erro::ErroConfig;
use crate::modelo::Config;
use crate::validacao::validar;

/// Carrega e valida a configuração a partir de um arquivo YAML.
///
/// # Errors
/// - [`ErroConfig::Leitura`] se o arquivo não puder ser lido;
/// - [`ErroConfig::Yaml`] se o conteúdo não casar com o esquema;
/// - [`ErroConfig::Validacao`] se alguma invariante de negócio for violada.
pub fn carregar_de_arquivo(caminho: impl AsRef<Path>) -> Result<Config, ErroConfig> {
    let caminho = caminho.as_ref();
    let conteudo = std::fs::read_to_string(caminho).map_err(|origem| ErroConfig::Leitura {
        caminho: caminho.display().to_string(),
        origem,
    })?;
    carregar_de_str(&conteudo)
}

/// Carrega e valida a configuração a partir de uma string YAML.
///
/// # Errors
/// - [`ErroConfig::Yaml`] se o conteúdo não casar com o esquema;
/// - [`ErroConfig::Validacao`] se alguma invariante de negócio for violada.
pub fn carregar_de_str(yaml: &str) -> Result<Config, ErroConfig> {
    let config: Config = serde_norway::from_str(yaml)?;
    validar(&config)?;
    Ok(config)
}
