mod support;

use std::sync::Arc;

use lambda_http::http::{Method, StatusCode};
use lambda_http::Body;
use shared::domain::{ArchiveStatus, InboxStatus, ItemKind, WatchStatus};
use shared::library::{
    InMemoryLibraryService, ItemTag, ItemUrlSummary, LibraryItemDetail, LibraryItemSummary,
};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use support::{
    assert_token_decodes, bearer_token, empty_library, request, response_json, test_app,
};

#[tokio::test]
async fn items_route_requires_auth() {
    let response = request(
        test_app(empty_library()),
        Method::GET,
        "/items",
        None,
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn items_route_returns_empty_library() {
    let auth = bearer_token("user-sub");
    assert_token_decodes(&auth, "user-sub");

    let response = request(
        test_app(empty_library()),
        Method::GET,
        "/items",
        Some(&auth),
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await, serde_json::json!([]));
}

#[tokio::test]
async fn items_route_returns_seeded_item_detail() {
    let item_id = item_id();
    let auth = bearer_token("user-sub");
    assert_token_decodes(&auth, "user-sub");

    let response = request(
        test_app(seeded_library(item_id)),
        Method::GET,
        &format!("/items/{item_id}"),
        Some(&auth),
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["summary"]["id"], item_id.to_string());
    assert_eq!(payload["summary"]["title"], "Saved video");
    assert_eq!(payload["summary"]["fetched_title"], "Fetched video");
    assert_eq!(payload["notes"], "watch for API shape");
}

#[tokio::test]
async fn items_route_returns_seeded_item_list() {
    let item_id = item_id();
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(seeded_library(item_id)),
        Method::GET,
        "/items",
        Some(&auth),
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload[0]["id"], item_id.to_string());
    assert_eq!(payload[0]["fetched_title"], "Fetched video");
    assert_eq!(payload[0]["watch_status"], "unwatched");
}

#[tokio::test]
async fn item_updates_route_returns_changed_items_as_a_batch() {
    let item_id = item_id();
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(seeded_library(item_id)),
        Method::GET,
        "/items/updates?since=1969-12-31T00%3A00%3A00Z&limit=10",
        Some(&auth),
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["items"][0]["id"], item_id.to_string());
    assert_eq!(payload["items"][0]["title"], "Saved video");
    assert_eq!(payload["items"][0]["fetched_title"], "Fetched video");
    assert_eq!(payload["deleted_item_ids"].as_array().unwrap().len(), 0);
    assert!(payload["cursor"].is_string() || payload["cursor"].is_array());
}

#[tokio::test]
async fn item_updates_route_without_since_returns_cursor_only() {
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(seeded_library(item_id())),
        Method::GET,
        "/items/updates",
        Some(&auth),
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["items"].as_array().unwrap().len(), 0);
    assert_eq!(payload["deleted_item_ids"].as_array().unwrap().len(), 0);
    assert!(payload["cursor"].is_string() || payload["cursor"].is_array());
}

#[tokio::test]
async fn item_updates_route_keeps_cursor_on_limited_batches() {
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(two_item_library()),
        Method::GET,
        "/items/updates?since=1969-12-31T00%3A00%3A00Z&limit=1",
        Some(&auth),
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    let cursor: OffsetDateTime = serde_json::from_value(payload["cursor"].clone()).unwrap();
    assert_eq!(payload["items"].as_array().unwrap().len(), 1);
    assert_eq!(cursor, OffsetDateTime::UNIX_EPOCH);
}

#[tokio::test]
async fn items_route_applies_library_filters() {
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(filtered_library()),
        Method::GET,
        "/items?platform=youtube&tag=learning&created_from=1969-12-31T00%3A00%3A00Z&created_to=1970-01-02T00%3A00%3A00Z&archive_status=succeeded&watch_status=unwatched&q=pipeline",
        Some(&auth),
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload.as_array().unwrap().len(), 1);
    assert_eq!(payload[0]["id"], "00000000-0000-0000-0000-000000000201");
}

#[tokio::test]
async fn items_route_filters_by_inbox_status() {
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(filtered_library()),
        Method::GET,
        "/items?inbox_status=organized",
        Some(&auth),
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload.as_array().unwrap().len(), 1);
    assert_eq!(payload[0]["id"], "00000000-0000-0000-0000-000000000202");
    assert_eq!(payload[0]["inbox_status"], "organized");
}

#[tokio::test]
async fn items_route_returns_not_found_shape_for_missing_item() {
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(empty_library()),
        Method::GET,
        &format!("/items/{}", item_id()),
        Some(&auth),
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], "not_found");
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
                fetched_title: Some("Fetched video".to_string()),
                thumbnail_s3_key: None,
                author: Some("Creator".to_string()),
                platform: Some("example".to_string()),
                duration_seconds: Some(42),
                archive_status: ArchiveStatus::Pending,
                watch_status: WatchStatus::Unwatched,
                inbox_status: InboxStatus::Unsorted,
                tags: vec![],
                created_at: OffsetDateTime::UNIX_EPOCH,
            },
            notes: "watch for API shape".to_string(),
        }],
    ))
}

fn filtered_library() -> Arc<InMemoryLibraryService> {
    Arc::new(InMemoryLibraryService::with_user_items(
        "user-sub",
        [
            filtered_item(
                Uuid::parse_str("00000000-0000-0000-0000-000000000201").unwrap(),
                "YouTube",
                InboxStatus::Unsorted,
            ),
            filtered_item(
                Uuid::parse_str("00000000-0000-0000-0000-000000000202").unwrap(),
                "TikTok",
                InboxStatus::Organized,
            ),
        ],
    ))
}

fn two_item_library() -> Arc<InMemoryLibraryService> {
    Arc::new(InMemoryLibraryService::with_user_items(
        "user-sub",
        [
            timed_item(
                Uuid::parse_str("00000000-0000-0000-0000-000000000301").unwrap(),
                OffsetDateTime::UNIX_EPOCH,
            ),
            timed_item(
                Uuid::parse_str("00000000-0000-0000-0000-000000000302").unwrap(),
                OffsetDateTime::UNIX_EPOCH + Duration::seconds(10),
            ),
        ],
    ))
}

fn timed_item(item_id: Uuid, created_at: OffsetDateTime) -> LibraryItemDetail {
    let mut item = filtered_item(item_id, "example", InboxStatus::Unsorted);
    item.summary.created_at = created_at;
    item
}

fn filtered_item(item_id: Uuid, platform: &str, inbox_status: InboxStatus) -> LibraryItemDetail {
    LibraryItemDetail {
        summary: LibraryItemSummary {
            id: item_id,
            item_kind: ItemKind::Url,
            url: Some(ItemUrlSummary::new(
                "https://example.com/original".to_string(),
                None,
            )),
            text: None,
            image: None,
            title: Some("Rust async talk".to_string()),
            fetched_title: None,
            thumbnail_s3_key: None,
            author: Some("Creator".to_string()),
            platform: Some(platform.to_string()),
            duration_seconds: Some(42),
            archive_status: ArchiveStatus::Succeeded,
            watch_status: WatchStatus::Unwatched,
            inbox_status,
            tags: vec![ItemTag {
                id: Uuid::parse_str("00000000-0000-0000-0000-000000000203").unwrap(),
                display_name: "Learning".to_string(),
                normalized_name: "learning".to_string(),
            }],
            created_at: OffsetDateTime::UNIX_EPOCH,
        },
        notes: "Rewatch for metadata pipeline notes".to_string(),
    }
}

fn item_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000101").unwrap()
}
