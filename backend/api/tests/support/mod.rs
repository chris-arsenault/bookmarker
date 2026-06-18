use std::sync::Arc;

use api::thumbnail_access::InMemoryThumbnailReader;
use api::{router, ApiState};
use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, Response};
use axum::Router;
use base64::Engine;
use serde_json::{json, Value};
use shared::auth::{decode_unverified_claims, AuthVerifier, UserContext};
use shared::config::{ApiConfig, AppConfig, CognitoConfig, DatabaseConfig};
use shared::db::database_url;
use shared::error::AppResult;
use shared::library::{InMemoryLibraryService, LibraryService};
use tower::ServiceExt;
use uuid::Uuid;

struct TestAuthVerifier;

#[async_trait]
impl AuthVerifier for TestAuthVerifier {
    async fn context_from_authorization(
        &self,
        auth_header: Option<&str>,
    ) -> AppResult<UserContext> {
        shared::auth::decode_unverified_claims(shared::auth::extract_bearer(auth_header)?)
    }
}

pub fn test_app(library: Arc<dyn LibraryService>) -> Router {
    test_app_with_processing_dispatcher(library, Arc::new(NoopProcessingDispatcher))
}

pub fn test_app_with_processing_dispatcher(
    library: Arc<dyn LibraryService>,
    processing_dispatcher: Arc<dyn api::processing_dispatch::ProcessingDispatcher>,
) -> Router {
    let config = test_config();
    let db = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy(&database_url(&config.database))
        .unwrap();
    router(ApiState::new(
        config,
        db,
        Arc::new(TestAuthVerifier),
        library,
        processing_dispatcher,
        Arc::new(InMemoryThumbnailReader::default()),
    ))
}

pub fn empty_library() -> Arc<dyn LibraryService> {
    Arc::new(InMemoryLibraryService::new())
}

pub fn bearer_token(sub: &str) -> String {
    let payload = json!({
        "sub": sub,
        "email": "chris@example.test",
        "cognito:username": "chris"
    });
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
    format!("Bearer {header}.{payload}.signature")
}

pub async fn request(
    app: Router,
    method: Method,
    uri: &str,
    auth: Option<&str>,
    body: Body,
) -> Response<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(auth) = auth {
        builder = builder.header("authorization", auth);
    }
    builder = builder.header("content-type", "application/json");
    app.oneshot(builder.body(body).unwrap()).await.unwrap()
}

pub async fn response_json(response: Response<Body>) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&body).unwrap()
}

pub fn assert_token_decodes(auth: &str, sub: &str) {
    let token = auth.strip_prefix("Bearer ").unwrap();
    let context = decode_unverified_claims(token).unwrap();
    assert_eq!(context.sub, sub);
}

#[async_trait]
impl api::processing_dispatch::ProcessingDispatcher for NoopProcessingDispatcher {
    async fn dispatch_item(&self, _item_id: Uuid) -> AppResult<()> {
        Ok(())
    }
}

struct NoopProcessingDispatcher;

fn test_config() -> AppConfig {
    AppConfig {
        database: DatabaseConfig {
            host: "localhost".to_string(),
            port: 5432,
            name: "linkdrop".to_string(),
            username: "linkdrop_app".to_string(),
            password: "password".to_string(),
        },
        api: ApiConfig {
            api_base_url: "https://api.example.test".to_string(),
            app_base_url: "https://app.example.test".to_string(),
        },
        cognito: CognitoConfig {
            user_pool_id: "us-east-1_pool".to_string(),
            client_id: "linkdrop-app-client".to_string(),
            domain: "auth.example.test".to_string(),
            issuer: "https://issuer.example.test".to_string(),
        },
    }
}
