use crate::config::AppConfig;
use crate::error::AppError;
use axum::{
    RequestPartsExt,
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use axum_extra::TypedHeader;
use axum_extra::headers::{Authorization, authorization::Basic};
use std::sync::Arc;
use subtle::ConstantTimeEq;

pub struct RequireAuth;

impl<S> FromRequestParts<S> for RequireAuth
where
    S: Send + Sync,
    Arc<AppConfig>: FromRef<S>,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let config = Arc::<AppConfig>::from_ref(state);

        if !config.auth.enabled {
            return Ok(RequireAuth);
        }

        let expected_user = config.auth.username.as_deref().unwrap_or("");
        let expected_pass = config.auth.password.as_deref().unwrap_or("");

        let TypedHeader(Authorization(auth)) = parts
            .extract::<TypedHeader<Authorization<Basic>>>()
            .await
            .map_err(|_| AppError::Unauthorized)?;

        // Use constant time comparison to prevent timing attacks
        let user_matches = auth.username().as_bytes().ct_eq(expected_user.as_bytes());
        let pass_matches = auth.password().as_bytes().ct_eq(expected_pass.as_bytes());

        if (user_matches & pass_matches).into() {
            Ok(RequireAuth)
        } else {
            Err(AppError::Unauthorized)
        }
    }
}
