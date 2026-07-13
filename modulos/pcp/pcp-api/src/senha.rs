//! Hash e verificação de senha com argon2id (CLAUDE.md §7.7).

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use rand_core::OsRng;

use crate::erro::ApiError;

/// Gera o hash argon2id de uma senha, com sal aleatório.
///
/// # Errors
/// [`ApiError::Interno`] se a derivação do hash falhar.
pub fn hashear(senha: &str) -> Result<String, ApiError> {
    let sal = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(senha.as_bytes(), &sal)
        .map_err(|_| ApiError::Interno)?;
    Ok(hash.to_string())
}

/// Verifica uma senha contra um hash argon2. `false` em qualquer divergência/hash inválido.
#[must_use]
pub fn verificar(senha: &str, hash: &str) -> bool {
    let Ok(analisado) = PasswordHash::new(hash) else {
        return false;
    };
    Argon2::default()
        .verify_password(senha.as_bytes(), &analisado)
        .is_ok()
}

#[cfg(test)]
mod testes {
    use super::{hashear, verificar};

    #[test]
    fn hash_e_verificacao() {
        let hash = hashear("segredo-forte-123").unwrap();
        assert!(verificar("segredo-forte-123", &hash));
        assert!(!verificar("senha-errada", &hash));
        assert!(!verificar("segredo-forte-123", "hash-invalido"));
    }
}
