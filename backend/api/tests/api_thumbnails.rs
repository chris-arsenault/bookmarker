use std::sync::Arc;

use api::image_access::InMemoryImageObjectStore;
use api::thumbnail_access::{InMemoryThumbnailReader, ThumbnailObject, ThumbnailReader};
use api::{router, ApiState, ApiStateServices};
use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Method, Request, Response, StatusCode};
use axum::Router;
use base64::Engine;
use serde_json::json;
use shared::auth::{AuthVerifier, UserContext};
use shared::config::{ApiConfig, AppConfig, CognitoConfig, DatabaseConfig};
use shared::db::database_url;
use shared::domain::{ArchiveStatus, InboxStatus, ItemKind, WatchStatus};
use shared::error::AppResult;
use shared::library::{
    InMemoryLibraryService, ItemUrlSummary, LibraryItemDetail, LibraryItemSummary,
};
use time::OffsetDateTime;
use tower::ServiceExt;
use uuid::Uuid;

#[tokio::test]
async fn item_thumbnail_route_returns_owned_snapshot() {
    let item_id = item_id();
    let thumbnail_reader = InMemoryThumbnailReader::from_objects([(
        "snapshots/item/thumbnail.jpg",
        ThumbnailObject {
            bytes: b"thumbnail-bytes".to_vec(),
            content_type: "image/jpeg".to_string(),
        },
    )]);
    let response = request(
        test_app_with_thumbnail_reader(seeded_library(item_id), Arc::new(thumbnail_reader)),
        Method::GET,
        &format!("/items/{item_id}/thumbnail"),
        Some(&bearer_token("user-sub")),
        Body::empty(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"].to_str().unwrap(),
        "image/jpeg"
    );
    assert_eq!(
        response.headers()["cache-control"].to_str().unwrap(),
        "private, max-age=31536000, immutable"
    );
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX).await.unwrap(),
        b"thumbnail-bytes".as_slice()
    );
}

fn seeded_library(item_id: Uuid) -> Arc<InMemoryLibraryService> {
    Arc::new(InMemoryLibraryService::with_user_items(
        "user-sub",
        [LibraryItemDetail {
            summary: LibraryItemSummary {
                id: item_id,
                item_kind: ItemKind::Url,
                url: Some(ItemUrlSummary::new(
                    "https://example.com/original".to_string(),
                    None,
                )),
                text: None,
                image: None,
                title: Some("Saved video".to_string()),
                fetched_title: None,
                thumbnail_s3_key: Some("snapshots/item/thumbnail.jpg".to_string()),
                author: Some("Creator".to_string()),
                platform: Some("example".to_string()),
                duration_seconds: Some(42),
                archive_status: ArchiveStatus::Succeeded,
                watch_status: WatchStatus::Unwatched,
                inbox_status: InboxStatus::Unsorted,
                tags: vec![],
                created_at: OffsetDateTime::UNIX_EPOCH,
            },
            notes: String::new(),
        }],
    ))
}

fn item_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000301").unwrap()
}

async fn request(
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
    app.oneshot(builder.body(body).unwrap()).await.unwrap()
}

fn test_app_with_thumbnail_reader(
    library: Arc<InMemoryLibraryService>,
    thumbnail_reader: Arc<dyn ThumbnailReader>,
) -> Router {
    let config = test_config();
    let db = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy(&database_url(&config.database))
        .unwrap();
    router(ApiState::new(
        config,
        db,
        ApiStateServices {
            auth: Arc::new(TestAuthVerifier),
            library,
            processing_dispatcher: Arc::new(NoopProcessingDispatcher),
            thumbnail_reader,
            image_store: Arc::new(InMemoryImageObjectStore),
        },
    ))
}

fn bearer_token(sub: &str) -> String {
    let payload = json!({
        "sub": sub,
        "email": "chris@example.test",
        "cognito:username": "chris"
    });
    let header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(r#"{"alg":"none"}"#);
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string());
    format!("Bearer {header}.{payload}.signature")
}

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

struct NoopProcessingDispatcher;

#[async_trait]
impl api::processing_dispatch::ProcessingDispatcher for NoopProcessingDispatcher {
    async fn dispatch_item(&self, _item_id: Uuid) -> AppResult<()> {
        Ok(())
    }
}

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
