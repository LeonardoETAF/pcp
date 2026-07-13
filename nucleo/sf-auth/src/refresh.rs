//! Refresh tokens revogáveis: geração e hash (CLAUDE.md §7.7).

use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};

/// Gera um refresh token: `(valor_bruto, hash)`. O bruto vai ao cliente; só o hash é
/// persistido (CLAUDE.md §7 — nunca guardar o token em claro).
#[must_use]
pub fn gerar_refresh() -> (String, String) {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let bruto = hex::encode(bytes);
    let hash = hash_refresh(&bruto);
    (bruto, hash)
}

/// Hash determinístico (SHA-256) de um refresh token, para lookup e revogação.
#[must_use]
pub fn hash_refresh(bruto: &str) -> String {
    hex::encode(Sha256::digest(bruto.as_bytes()))
}

#[cfg(test)]
mod testes {
    use super::{gerar_refresh, hash_refresh};

    #[test]
    fn refresh_unico_e_hash_estavel() {
        let (bruto1, hash1) = gerar_refresh();
        let (bruto2, _) = gerar_refresh();
        assert_ne!(bruto1, bruto2);
        assert_eq!(hash1, hash_refresh(&bruto1));
        assert_ne!(bruto1, hash1);
    }
}
