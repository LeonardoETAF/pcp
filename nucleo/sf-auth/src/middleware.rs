//! Middleware de autenticação **deny-by-default** (CLAUDE.md §7.1), genérico no estado.
//!
//! O middleware não conhece o `AppState` de nenhum módulo: ele só exige que o estado saiba
//! entregar o [`SegredoJwt`], via `FromRef` do Axum. Cada módulo (PCP, Catálogo, ...) implementa
//! esse `FromRef` no seu próprio estado — e assim **nenhum módulo depende de outro** (§0).

use std::sync::Arc;

use axum::extract::{FromRef, Request, State};
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::Response;
use sf_http::ApiError;

use crate::jwt;

/// Segredo de assinatura do JWT, extraível de qualquer `AppState` que implemente `FromRef`.
#[derive(Clone)]
pub struct SegredoJwt(pub Arc<Vec<u8>>);

/// Exige `Authorization: Bearer <token>` válido e injeta as [`Claims`](crate::Claims) nas
/// extensões da requisição. Aplique-o ao subgrupo protegido inteiro — nunca rota a rota
/// (deny-by-default, CLAUDE.md §7.1).
///
/// # Errors
/// [`ApiError::NaoAutenticado`] se o token faltar ou for inválido/expirado.
pub async fn exigir_autenticacao<S>(
    State(estado): State<S>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError>
where
    S: Send + Sync + 'static,
    SegredoJwt: FromRef<S>,
{
    let SegredoJwt(segredo) = SegredoJwt::from_ref(&estado);
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .ok_or(ApiError::NaoAutenticado)?;
    let claims = jwt::decodificar(token, &segredo)?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}
