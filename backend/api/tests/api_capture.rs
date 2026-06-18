mod support;

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::body::Body;
use axum::http::{Method, StatusCode};
use shared::error::{AppError, AppResult};
use shared::library::InMemoryLibraryService;
use uuid::Uuid;

use support::{
    assert_token_decodes, bearer_token, empty_library, request, response_json, test_app,
    test_app_with_processing_dispatcher,
};

#[tokio::test]
async fn capture_route_requires_auth() {
    let response = request(
        test_app(empty_library()),
        Method::POST,
        "/items",
        None,
        Body::from(r#"{"url":"https://example.com/watch"}"#),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], "unauthorized");
}

#[tokio::test]
async fn capture_route_rejects_invalid_url_with_validation_error() {
    let auth = bearer_token("user-sub");
    assert_token_decodes(&auth, "user-sub");

    let response = request(
        test_app(empty_library()),
        Method::POST,
        "/items",
        Some(&auth),
        Body::from(r#"{"url":"not-a-url"}"#),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], "validation_error");
}

#[tokio::test]
async fn capture_route_accepts_url_without_tags_and_lists_item() {
    let library = Arc::new(InMemoryLibraryService::new());
    let auth = bearer_token("user-sub");

    let response = request(
        test_app(library.clone()),
        Method::POST,
        "/items",
        Some(&auth),
        Body::from(
            r#"{"url":"https://example.com/watch?utm_source=share","client_capture_id":"capture-1"}"#,
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload = response_json(response).await;
    assert_eq!(
        payload["item"]["summary"]["url"]["original_url"],
        "https://example.com/watch?utm_source=share"
    );
    assert_eq!(
        payload["item"]["summary"]["url"]["canonical_url"],
        "https://example.com/watch"
    );
    assert_eq!(
        payload["item"]["summary"]["url"]["copy_url"],
        "https://example.com/watch"
    );

    let response = request(
        test_app(library),
        Method::GET,
        "/items",
        Some(&auth),
        Body::empty(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload.as_array().unwrap().len(), 1);
    assert_eq!(
        payload[0]["url"]["original_url"],
        "https://example.com/watch?utm_source=share"
    );
    assert_eq!(payload[0]["url"]["copy_url"], "https://example.com/watch");
}

#[tokio::test]
async fn text_capture_route_accepts_snippet_and_lists_item() {
    let library = Arc::new(InMemoryLibraryService::new());
    let auth = bearer_token("user-sub");

    let response = request(
        test_app(library.clone()),
        Method::POST,
        "/items/text",
        Some(&auth),
        Body::from(
            r#"{"plain_text":"copy this for later","source_app":"Terminal","tags":["Shell"],"client_capture_id":"text-1"}"#,
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload = response_json(response).await;
    assert_eq!(payload["item"]["summary"]["item_kind"], "text_snippet");
    assert_eq!(
        payload["item"]["summary"]["text"]["plain_text"],
        "copy this for later"
    );
    assert_eq!(payload["item"]["summary"]["text"]["source_app"], "Terminal");
    assert_eq!(
        payload["item"]["summary"]["archive_status"],
        "not_applicable"
    );

    let response = request(
        test_app(library),
        Method::GET,
        "/items?q=copy",
        Some(&auth),
        Body::empty(),
    )
    .await;
    let payload = response_json(response).await;
    assert_eq!(payload.as_array().unwrap().len(), 1);
    assert_eq!(payload[0]["text"]["preview"], "copy this for later");
}

#[tokio::test]
async fn capture_route_returns_unsorted_item() {
    let library = Arc::new(InMemoryLibraryService::new());
    let auth = bearer_token("user-sub");

    let response = request(
        test_app(library),
        Method::POST,
        "/items",
        Some(&auth),
        Body::from(r#"{"url":"https://example.com/watch","client_capture_id":"capture-inbox"}"#),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload = response_json(response).await;
    assert_eq!(payload["item"]["summary"]["inbox_status"], "unsorted");
}

#[tokio::test]
async fn capture_route_applies_explicit_tags() {
    let library = Arc::new(InMemoryLibraryService::new());
    let auth = bearer_token("user-sub");

    let response = request(
        test_app(library.clone()),
        Method::POST,
        "/items",
        Some(&auth),
        Body::from(
            r#"{"url":"https://example.com/watch","tags":[" Learning ","learning"],"client_capture_id":"capture-tags"}"#,
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload = response_json(response).await;
    assert_eq!(
        payload["item"]["summary"]["tags"][0]["normalized_name"],
        "learning"
    );

    let response = request(
        test_app(library),
        Method::GET,
        "/tags",
        Some(&auth),
        Body::empty(),
    )
    .await;

    let payload = response_json(response).await;
    assert_eq!(payload[0]["display_name"], "Learning");
    assert_eq!(payload[0]["usage_count"], 1);
}

#[tokio::test]
async fn capture_route_reuses_client_capture_id() {
    let library = Arc::new(InMemoryLibraryService::new());
    let auth = bearer_token("user-sub");
    let body = r#"{"url":"https://example.com/watch","client_capture_id":"capture-retry"}"#;

    let first = request(
        test_app(library.clone()),
        Method::POST,
        "/items",
        Some(&auth),
        Body::from(body),
    )
    .await;
    let first_payload = response_json(first).await;

    let second = request(
        test_app(library.clone()),
        Method::POST,
        "/items",
        Some(&auth),
        Body::from(body),
    )
    .await;

    assert_eq!(second.status(), StatusCode::OK);
    let second_payload = response_json(second).await;
    assert_eq!(
        first_payload["item"]["summary"]["id"],
        second_payload["item"]["summary"]["id"]
    );

    let response = request(
        test_app(library),
        Method::GET,
        "/items",
        Some(&auth),
        Body::empty(),
    )
    .await;
    assert_eq!(response_json(response).await.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn capture_route_returns_existing_item_for_normalized_repeat() {
    let library = Arc::new(InMemoryLibraryService::new());
    let auth = bearer_token("user-sub");

    let first = request(
        test_app(library.clone()),
        Method::POST,
        "/items",
        Some(&auth),
        Body::from(
            r#"{"url":"https://youtu.be/video-id?utm_source=share&t=42","client_capture_id":"capture-normalized-1"}"#,
        ),
    )
    .await;
    let first_payload = response_json(first).await;

    let second = request(
        test_app(library.clone()),
        Method::POST,
        "/items",
        Some(&auth),
        Body::from(
            r#"{"url":"https://www.youtube.com/watch?v=video-id&t=42&utm_campaign=again","client_capture_id":"capture-normalized-2"}"#,
        ),
    )
    .await;

    assert_eq!(second.status(), StatusCode::OK);
    let second_payload = response_json(second).await;
    assert_eq!(second_payload["created"], false);
    assert_eq!(
        first_payload["item"]["summary"]["id"],
        second_payload["item"]["summary"]["id"]
    );
    assert_eq!(
        second_payload["item"]["summary"]["url"]["canonical_url"],
        "https://www.youtube.com/watch?v=video-id&t=42"
    );
    assert_eq!(
        second_payload["item"]["summary"]["url"]["copy_url"],
        "https://www.youtube.com/watch?v=video-id&t=42"
    );

    let response = request(
        test_app(library),
        Method::GET,
        "/items",
        Some(&auth),
        Body::empty(),
    )
    .await;
    assert_eq!(response_json(response).await.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn processing_dispatch_runs_for_created_and_retry_captures() {
    let library = Arc::new(InMemoryLibraryService::new());
    let dispatcher = Arc::new(RecordingProcessingDispatcher::default());
    let auth = bearer_token("user-sub");
    let body = r#"{"url":"https://example.com/watch","client_capture_id":"dispatch-retry"}"#;

    let first = request(
        test_app_with_processing_dispatcher(library.clone(), dispatcher.clone()),
        Method::POST,
        "/items",
        Some(&auth),
        Body::from(body),
    )
    .await;
    let first_payload = response_json(first).await;
    let item_id = first_payload["item"]["summary"]["id"]
        .as_str()
        .and_then(|value| Uuid::parse_str(value).ok())
        .unwrap();

    let second = request(
        test_app_with_processing_dispatcher(library, dispatcher.clone()),
        Method::POST,
        "/items",
        Some(&auth),
        Body::from(body),
    )
    .await;

    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(dispatcher.item_ids(), vec![item_id, item_id]);
}

#[tokio::test]
async fn processing_dispatch_failure_does_not_block_capture() {
    let library = Arc::new(InMemoryLibraryService::new());
    let auth = bearer_token("user-sub");

    let response = request(
        test_app_with_processing_dispatcher(library, Arc::new(FailingProcessingDispatcher)),
        Method::POST,
        "/items",
        Some(&auth),
        Body::from(r#"{"url":"https://example.com/watch","client_capture_id":"dispatch-fail"}"#),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload = response_json(response).await;
    assert_eq!(payload["created"], true);
    assert_eq!(
        payload["item"]["summary"]["url"]["copy_url"],
        "https://example.com/watch"
    );
}

#[derive(Default)]
struct RecordingProcessingDispatcher {
    item_ids: Mutex<Vec<Uuid>>,
}

impl RecordingProcessingDispatcher {
    fn item_ids(&self) -> Vec<Uuid> {
        self.item_ids.lock().unwrap().clone()
    }
}

#[async_trait]
impl api::processing_dispatch::ProcessingDispatcher for RecordingProcessingDispatcher {
    async fn dispatch_item(&self, item_id: Uuid) -> AppResult<()> {
        self.item_ids.lock().unwrap().push(item_id);
        Ok(())
    }
}

struct FailingProcessingDispatcher;

#[async_trait]
impl api::processing_dispatch::ProcessingDispatcher for FailingProcessingDispatcher {
    async fn dispatch_item(&self, _item_id: Uuid) -> AppResult<()> {
        Err(AppError::Internal("dispatch unavailable".to_string()))
    }
}
