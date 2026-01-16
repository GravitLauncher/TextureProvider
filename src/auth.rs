use std::sync::Arc;

use crate::models::JwtClaims;
use anyhow::Result;
use axum::http::{HeaderMap, StatusCode};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use uuid::Uuid;

/// Extract and validate JWT from Authorization header
pub fn extract_jwt(headers: &HeaderMap) -> Result<String> {
    let auth_header = headers
        .get("authorization")
        .ok_or_else(|| anyhow::anyhow!("Missing authorization header"))?
        .to_str()
        .map_err(|_| anyhow::anyhow!("Invalid authorization header"))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(anyhow::anyhow!("Invalid authorization header format"));
    }

    let token = auth_header[7..].to_string();
    Ok(token)
}

pub fn decode_key(public_key: &str) -> Result<DecodingKey> {
    let key = format!(
        "-----BEGIN PUBLIC KEY-----\n{}\n-----END PUBLIC KEY-----",
        public_key
    );
    DecodingKey::from_ec_pem(key.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to create decoding key: {}", e))
}

/// Decode and validate JWT token, returning user UUID
pub fn validate_jwt(token: &str, public_key: &DecodingKey) -> Result<Uuid> {
    let mut validation = Validation::new(Algorithm::ES256);
    validation.validate_exp = true;

    let token_data = decode::<JwtClaims>(token, public_key, &validation)
        .map_err(|e| anyhow::anyhow!("Invalid JWT token: {}", e))?;

    let uuid = Uuid::parse_str(&token_data.claims.uuid)
        .map_err(|e| anyhow::anyhow!("Invalid UUID in token: {}", e))?;

    Ok(uuid)
}

/// Extract user UUID from validated JWT
pub struct AuthUser(pub Uuid);

impl std::fmt::Debug for AuthUser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AuthUser").field(&self.0).finish()
    }
}

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Get public key from request extensions (set by middleware)
        let public_key = parts
            .extensions
            .get::<Arc<DecodingKey>>()
            .ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Public key not found in state".to_string(),
                )
            })?
            .clone();

        let token = extract_jwt(&parts.headers).map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                format!("Authentication failed: {}", e),
            )
        })?;

        let uuid = validate_jwt(&token, &public_key).map_err(|e| {
            (
                StatusCode::UNAUTHORIZED,
                format!("Authentication failed: {}", e),
            )
        })?;

        Ok(AuthUser(uuid))
    }
}

/// Extract admin token from Authorization header
/// Marker struct to indicate admin authentication is required
pub struct AuthAdmin;

impl std::fmt::Debug for AuthAdmin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("AuthAdmin").finish()
    }
}

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AuthAdmin
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("authorization")
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    "Missing authorization header".to_string(),
                )
            })?
            .to_str()
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    "Invalid authorization header".to_string(),
                )
            })?;

        if !auth_header.starts_with("Bearer ") {
            return Err((
                StatusCode::UNAUTHORIZED,
                "Invalid authorization header format".to_string(),
            ));
        }

        let token = auth_header[7..].to_string();

        // Get admin token from request extensions (set by middleware)
        let admin_token = parts
            .extensions
            .get::<String>()
            .and_then(|t| {
                if t.starts_with("admin_token:") {
                    Some(t.clone())
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Admin token not configured".to_string(),
                )
            })?;

        // Extract the actual token value (remove "admin_token:" prefix)
        let expected_token = admin_token
            .strip_prefix("admin_token:")
            .unwrap_or(&admin_token);

        if token != expected_token {
            return Err((StatusCode::UNAUTHORIZED, "Invalid admin token".to_string()));
        }

        Ok(AuthAdmin)
    }
}
