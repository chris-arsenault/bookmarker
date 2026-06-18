use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use base64::Engine;
use jsonwebtoken::jwk::JwkSet;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

use crate::config::CognitoConfig;
use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserContext {
    pub sub: String,
    pub email: Option<String>,
    pub username: Option<String>,
    pub groups: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Claims {
    sub: String,
    email: Option<String>,
    username: Option<String>,
    #[serde(rename = "cognito:username")]
    cognito_username: Option<String>,
    #[serde(default, rename = "cognito:groups")]
    cognito_groups: Vec<String>,
    token_use: Option<String>,
    client_id: Option<String>,
}

#[async_trait]
pub trait AuthVerifier: Send + Sync {
    async fn context_from_authorization(&self, auth_header: Option<&str>)
        -> AppResult<UserContext>;
}

#[async_trait]
pub trait JwksProvider: Send + Sync {
    async fn jwks(&self) -> AppResult<Arc<JwkSet>>;
}

#[derive(Clone)]
pub struct CognitoJwtVerifier {
    issuer: String,
    client_id: String,
    jwks_provider: Arc<dyn JwksProvider>,
}

impl CognitoJwtVerifier {
    pub fn from_config(config: &CognitoConfig) -> Self {
        Self::new(
            config.issuer.clone(),
            config.client_id.clone(),
            Arc::new(HttpJwksProvider::new(&config.issuer)),
        )
    }

    pub fn new(
        issuer: impl Into<String>,
        client_id: impl Into<String>,
        jwks_provider: Arc<dyn JwksProvider>,
    ) -> Self {
        Self {
            issuer: issuer.into(),
            client_id: client_id.into(),
            jwks_provider,
        }
    }

    async fn verify_token(&self, token: &str) -> AppResult<UserContext> {
        let header = decode_header(token)
            .map_err(|_| AppError::Unauthorized("invalid bearer token".to_string()))?;
        let kid = header
            .kid
            .ok_or_else(|| AppError::Unauthorized("bearer token is missing key id".to_string()))?;
        let jwks = self.jwks_provider.jwks().await?;
        let jwk = jwks
            .find(&kid)
            .ok_or_else(|| AppError::Unauthorized("unknown bearer token key".to_string()))?;
        let decoding_key = DecodingKey::from_jwk(jwk)
            .map_err(|_| AppError::Unauthorized("invalid bearer token key".to_string()))?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_required_spec_claims(&["exp", "iss", "sub"]);
        validation.validate_aud = false;

        let claims = decode::<Claims>(token, &decoding_key, &validation)
            .map_err(|_| AppError::Unauthorized("invalid bearer token".to_string()))?
            .claims;
        validate_access_claims(&claims, &self.client_id)?;
        Ok(claims.into_user_context())
    }
}

#[async_trait]
impl AuthVerifier for CognitoJwtVerifier {
    async fn context_from_authorization(
        &self,
        auth_header: Option<&str>,
    ) -> AppResult<UserContext> {
        self.verify_token(extract_bearer(auth_header)?).await
    }
}

pub struct HttpJwksProvider {
    url: String,
    client: reqwest::Client,
    cached: Mutex<Option<Arc<JwkSet>>>,
}

impl HttpJwksProvider {
    pub fn new(issuer: &str) -> Self {
        Self {
            url: format!("{}/.well-known/jwks.json", issuer.trim_end_matches('/')),
            client: reqwest::Client::new(),
            cached: Mutex::new(None),
        }
    }
}

#[async_trait]
impl JwksProvider for HttpJwksProvider {
    async fn jwks(&self) -> AppResult<Arc<JwkSet>> {
        if let Some(cached) = self.cached.lock().unwrap().clone() {
            return Ok(cached);
        }

        let response =
            self.client
                .get(&self.url)
                .send()
                .await
                .map_err(|err| AppError::ExternalService {
                    service: "cognito_jwks",
                    message: err.to_string(),
                })?;
        if !response.status().is_success() {
            return Err(AppError::ExternalService {
                service: "cognito_jwks",
                message: format!("JWKS fetch returned HTTP {}", response.status()),
            });
        }

        let jwks =
            Arc::new(
                response
                    .json::<JwkSet>()
                    .await
                    .map_err(|err| AppError::ExternalService {
                        service: "cognito_jwks",
                        message: err.to_string(),
                    })?,
            );
        *self.cached.lock().unwrap() = Some(jwks.clone());
        Ok(jwks)
    }
}

pub fn extract_bearer(auth_header: Option<&str>) -> AppResult<&str> {
    let header =
        auth_header.ok_or_else(|| AppError::Unauthorized("missing Authorization header".into()))?;

    header
        .strip_prefix("Bearer ")
        .or_else(|| header.strip_prefix("bearer "))
        .ok_or_else(|| AppError::Unauthorized("missing Bearer token".into()))
}

pub fn decode_unverified_claims(token: &str) -> AppResult<UserContext> {
    let payload_b64 = token
        .split('.')
        .nth(1)
        .ok_or_else(|| AppError::Unauthorized("malformed token".into()))?;
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| AppError::Unauthorized("invalid token encoding".into()))?;
    let claims: Claims = serde_json::from_slice(&payload_bytes)
        .map_err(|_| AppError::Unauthorized("invalid token claims".into()))?;

    Ok(claims.into_user_context())
}

fn validate_access_claims(claims: &Claims, client_id: &str) -> AppResult<()> {
    if claims.token_use.as_deref() != Some("access") {
        return Err(AppError::Unauthorized(
            "bearer token is not an access token".to_string(),
        ));
    }
    if claims.client_id.as_deref() != Some(client_id) {
        return Err(AppError::Unauthorized(
            "bearer token was issued for another client".to_string(),
        ));
    }
    Ok(())
}

impl Claims {
    fn into_user_context(self) -> UserContext {
        UserContext {
            sub: self.sub,
            email: self.email,
            username: self.username.or(self.cognito_username),
            groups: self.cognito_groups,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use base64::Engine;
    use serde_json::json;

    use crate::error::AppError;

    use super::{decode_unverified_claims, extract_bearer};

    fn token(payload: serde_json::Value) -> String {
        let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
        let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
        format!("{header}.{payload}.signature")
    }

    #[test]
    fn extracts_bearer_token() {
        assert_eq!(
            extract_bearer(Some("Bearer abc.def.ghi")).unwrap(),
            "abc.def.ghi"
        );
    }

    #[test]
    fn extracts_lowercase_bearer_token() {
        assert_eq!(
            extract_bearer(Some("bearer abc.def.ghi")).unwrap(),
            "abc.def.ghi"
        );
    }

    #[test]
    fn rejects_missing_bearer_token() {
        let err = extract_bearer(Some("Basic abc")).unwrap_err();

        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[test]
    fn rejects_malformed_token() {
        let err = decode_unverified_claims("not-a-jwt").unwrap_err();

        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[test]
    fn decodes_unverified_cognito_style_token_payload() {
        let context = decode_unverified_claims(&token(json!({
            "sub": "cognito-sub",
            "email": "chris@example.test",
            "cognito:username": "chris",
            "cognito:groups": ["admin", "linkdrop"]
        })))
        .unwrap();

        assert_eq!(context.sub, "cognito-sub");
        assert_eq!(context.email.as_deref(), Some("chris@example.test"));
        assert_eq!(context.username.as_deref(), Some("chris"));
        assert_eq!(context.groups, ["admin", "linkdrop"]);
    }
}
