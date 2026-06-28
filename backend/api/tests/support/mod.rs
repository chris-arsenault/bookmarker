use std::sync::Arc;

use api::image_access::{ImageObjectStore, InMemoryImageObjectStore};
use api::thumbnail_access::{InMemoryThumbnailReader, ThumbnailReader};
use api::{handle_request, ApiState, ApiStateServices};
use async_trait::async_trait;
use base64::Engine;
use lambda_http::http::Method;
use lambda_http::{Body, Response};
use serde_json::{json, Value};
use shared::auth::{decode_unverified_claims, AuthVerifier, UserContext};
use shared::config::{ApiConfig, AppConfig, CognitoConfig, DatabaseConfig};
use shared::db::database_url;
use shared::error::AppResult;
use shared::library::{InMemoryLibraryService, LibraryService};
use uuid::Uuid;

pub type TestApp = Arc<ApiState>;

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

#[allow(dead_code)]
pub fn test_app(library: Arc<dyn LibraryService>) -> TestApp {
    test_app_with_processing_dispatcher(library, Arc::new(NoopProcessingDispatcher))
}

#[allow(dead_code)]
pub fn test_app_with_image_store(
    library: Arc<dyn LibraryService>,
    image_store: Arc<dyn ImageObjectStore>,
) -> TestApp {
    test_app_with_state_parts(
        library,
        Arc::new(NoopProcessingDispatcher),
        Arc::new(InMemoryThumbnailReader::default()),
        image_store,
    )
}

#[allow(dead_code)]
pub fn test_app_with_thumbnail_reader(
    library: Arc<dyn LibraryService>,
    thumbnail_reader: Arc<dyn ThumbnailReader>,
) -> TestApp {
    test_app_with_state_parts(
        library,
        Arc::new(NoopProcessingDispatcher),
        thumbnail_reader,
        Arc::new(InMemoryImageObjectStore),
    )
}

#[allow(dead_code)]
pub fn test_app_with_processing_dispatcher(
    library: Arc<dyn LibraryService>,
    processing_dispatcher: Arc<dyn api::processing_dispatch::ProcessingDispatcher>,
) -> TestApp {
    test_app_with_state_parts(
        library,
        processing_dispatcher,
        Arc::new(InMemoryThumbnailReader::default()),
        Arc::new(InMemoryImageObjectStore),
    )
}

fn test_app_with_state_parts(
    library: Arc<dyn LibraryService>,
    processing_dispatcher: Arc<dyn api::processing_dispatch::ProcessingDispatcher>,
    thumbnail_reader: Arc<dyn ThumbnailReader>,
    image_store: Arc<dyn ImageObjectStore>,
) -> TestApp {
    let config = test_config();
    let db = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy(&database_url(&config.database))
        .unwrap();
    Arc::new(ApiState::new(
        config,
        db,
        ApiStateServices {
            auth: Arc::new(TestAuthVerifier),
            library,
            processing_dispatcher,
            thumbnail_reader,
            image_store,
        },
    ))
}

#[allow(dead_code)]
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
    app: TestApp,
    method: Method,
    uri: &str,
    auth: Option<&str>,
    body: Body,
) -> Response<Body> {
    let mut builder = lambda_http::http::Request::builder()
        .method(method)
        .uri(uri);
    if let Some(auth) = auth {
        builder = builder.header("authorization", auth);
    }
    builder = builder.header("content-type", "application/json");
    handle_request(builder.body(body).unwrap(), app).await
}

#[allow(dead_code)]
pub async fn response_json(response: Response<Body>) -> Value {
    serde_json::from_slice(api::http_body_bytes(response.body())).unwrap()
}

#[allow(dead_code)]
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
