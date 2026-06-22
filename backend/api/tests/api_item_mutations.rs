mod support;

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Method, StatusCode};
use shared::domain::{ArchiveStatus, InboxStatus, ItemKind, WatchStatus};
use shared::library::{
    InMemoryLibraryService, ItemTag, ItemUrlSummary, LibraryItemDetail, LibraryItemSummary,
};
use time::OffsetDateTime;
use uuid::Uuid;

use support::{
    assert_token_decodes, bearer_token, empty_library, request, response_json, test_app,
};

#[tokio::test]
async fn patch_item_route_requires_auth() {
    let response = request(
        test_app(empty_library()),
        Method::PATCH,
        &format!("/items/{}", item_id()),
        None,
        Body::from(r#"{"watch_status":"watched"}"#),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], "unauthorized");
}

#[tokio::test]
async fn delete_item_route_requires_auth() {
    let response = request(
        test_app(empty_library()),
        Method::DELETE,
        &format!("/items/{}", item_id()),
        None,
        Body::empty(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], "unauthorized");
}

#[tokio::test]
async fn patch_item_route_updates_watch_status() {
    let item_id = item_id();
    let auth = bearer_token("user-sub");
    assert_token_decodes(&auth, "user-sub");

    let response = request(
        test_app(seeded_library(item_id)),
        Method::PATCH,
        &format!("/items/{item_id}"),
        Some(&auth),
        Body::from(r#"{"watch_status":"watched"}"#),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["summary"]["id"], item_id.to_string());
    assert_eq!(payload["summary"]["watch_status"], "watched");
}

#[tokio::test]
async fn patch_item_route_edits_tags_notes_watch_and_inbox() {
    let item_id = item_id();
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(seeded_library(item_id)),
        Method::PATCH,
        &format!("/items/{item_id}"),
        Some(&auth),
        Body::from(
            r#"{
                "watch_status":"watched",
                "inbox_status":"organized",
                "notes":"Filed after watching",
                "tags":[" Learning ","Videos"]
            }"#,
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["summary"]["watch_status"], "watched");
    assert_eq!(payload["summary"]["inbox_status"], "organized");
    assert_eq!(payload["notes"], "Filed after watching");
    assert_eq!(payload["summary"]["tags"][0]["display_name"], "Learning");
    assert_eq!(payload["summary"]["tags"][1]["display_name"], "Videos");
}

#[tokio::test]
async fn patch_item_route_edits_item_title() {
    let item_id = item_id();
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(seeded_library(item_id)),
        Method::PATCH,
        &format!("/items/{item_id}"),
        Some(&auth),
        Body::from(r#"{"title":"Renamed saved item"}"#),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["summary"]["title"], "Renamed saved item");
}

#[tokio::test]
async fn patch_item_route_rejects_empty_organization_update() {
    let item_id = item_id();
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(seeded_library(item_id)),
        Method::PATCH,
        &format!("/items/{item_id}"),
        Some(&auth),
        Body::from(r#"{}"#),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], "validation_error");
}

#[tokio::test]
async fn delete_item_route_removes_item() {
    let item_id = item_id();
    let auth = bearer_token("user-sub");
    let app = test_app(seeded_library(item_id));

    let response = request(
        app.clone(),
        Method::DELETE,
        &format!("/items/{item_id}"),
        Some(&auth),
        Body::empty(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let response = request(
        app,
        Method::GET,
        &format!("/items/{item_id}"),
        Some(&auth),
        Body::empty(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
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
                thumbnail_s3_key: None,
                author: Some("Creator".to_string()),
                platform: Some("example".to_string()),
                duration_seconds: Some(42),
                archive_status: ArchiveStatus::Pending,
                watch_status: WatchStatus::Unwatched,
                inbox_status: InboxStatus::Unsorted,
                tags: vec![ItemTag {
                    id: Uuid::parse_str("00000000-0000-0000-0000-000000000103").unwrap(),
                    display_name: "Learning".to_string(),
                    normalized_name: "learning".to_string(),
                }],
                created_at: OffsetDateTime::UNIX_EPOCH,
            },
            notes: String::new(),
        }],
    ))
}

fn item_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000102").unwrap()
}
