//! Bootstrap do admin inicial a partir do ambiente (CLAUDE.md §7.3/§7.4).

use pcp_db::{usuarios, PgPool};

use crate::estado::ErroBootstrap;
use crate::senha;

/// Cria um admin inicial se ainda não houver nenhum usuário. A senha vem do ambiente.
///
/// # Errors
/// [`ErroBootstrap::Db`] em falha de banco; [`ErroBootstrap::Hashing`] se o hash falhar.
pub async fn garantir_admin_inicial(
    pool: &PgPool,
    email: &str,
    senha_clara: &str,
) -> Result<(), ErroBootstrap> {
    if usuarios::contar(pool).await? > 0 {
        return Ok(());
    }
    let hash = senha::hashear(senha_clara).map_err(|_| ErroBootstrap::Hashing)?;
    let email = email.trim().to_lowercase();
    usuarios::criar(pool, &email, &hash, "admin", Some("Admin inicial")).await?;
    tracing::info!(%email, "admin inicial criado");
    Ok(())
}
