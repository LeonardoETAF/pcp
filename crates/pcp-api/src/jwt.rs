//! Tokens de acesso JWT (HS256) e claims (CLAUDE.md §7.7).

use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::erro::ApiError;
use crate::papel::Papel;

/// Claims do token de acesso. `sub` = id do usuário; `iat`/`exp` em segundos Unix.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub papel: String,
    pub iat: i64,
    pub exp: i64,
}

impl Claims {
    /// Papel convertido do texto, se válido.
    #[must_use]
    pub fn papel(&self) -> Option<Papel> {
        Papel::tentar_de(&self.papel)
    }

    /// Garante que o papel do token é pelo menos `minimo`.
    ///
    /// # Errors
    /// [`ApiError::Proibido`] se o privilégio for insuficiente ou o papel inválido.
    pub fn exige(&self, minimo: Papel) -> Result<(), ApiError> {
        match self.papel() {
            Some(p) if p.pelo_menos(minimo) => Ok(()),
            _ => Err(ApiError::Proibido),
        }
    }
}

/// Gera um token de acesso assinado para o usuário.
///
/// # Errors
/// [`ApiError::Interno`] se a codificação falhar.
pub fn gerar_access(
    sub: &str,
    papel: &str,
    segredo: &[u8],
    ttl: Duration,
) -> Result<String, ApiError> {
    let agora = Utc::now();
    let claims = Claims {
        sub: sub.to_owned(),
        papel: papel.to_owned(),
        iat: agora.timestamp(),
        exp: (agora + ttl).timestamp(),
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(segredo),
    )
    .map_err(|_| ApiError::Interno)
}

/// Decodifica e valida (assinatura + expiração) um token de acesso.
///
/// # Errors
/// [`ApiError::NaoAutenticado`] se o token for inválido ou expirado.
pub fn decodificar(token: &str, segredo: &[u8]) -> Result<Claims, ApiError> {
    let dados = decode::<Claims>(
        token,
        &DecodingKey::from_secret(segredo),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|_| ApiError::NaoAutenticado)?;
    Ok(dados.claims)
}

#[cfg(test)]
mod testes {
    use super::{decodificar, gerar_access};
    use crate::papel::Papel;
    use chrono::Duration;

    const SEGREDO: &[u8] = b"segredo-de-teste-com-mais-de-32-bytes!!";

    #[test]
    fn round_trip_valido() {
        let token = gerar_access("u1", "gestor", SEGREDO, Duration::minutes(15)).unwrap();
        let claims = decodificar(&token, SEGREDO).unwrap();
        assert_eq!(claims.sub, "u1");
        assert_eq!(claims.papel(), Some(Papel::Gestor));
        assert!(claims.exige(Papel::Analista).is_ok());
        assert!(claims.exige(Papel::Admin).is_err());
    }

    #[test]
    fn rejeita_expirado() {
        let token = gerar_access("u1", "admin", SEGREDO, Duration::minutes(-2)).unwrap();
        assert!(decodificar(&token, SEGREDO).is_err());
    }

    #[test]
    fn rejeita_segredo_errado() {
        let token = gerar_access("u1", "admin", SEGREDO, Duration::minutes(15)).unwrap();
        assert!(decodificar(&token, b"outro-segredo-totalmente-diferente!!").is_err());
    }
}
