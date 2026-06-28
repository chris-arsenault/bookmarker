mod support;

use std::sync::Arc;

use lambda_http::http::{Method, StatusCode};
use lambda_http::Body;
use shared::domain::{ArchiveStatus, InboxStatus, ItemKind, WatchStatus};
use shared::library::{
    InMemoryLibraryService, ItemTag, ItemUrlSummary, LibraryItemDetail, LibraryItemSummary,
    TagCorpusEntry,
};
use time::OffsetDateTime;
use uuid::Uuid;

use support::{
    assert_token_decodes, bearer_token, empty_library, request, response_json, test_app,
};

#[tokio::test]
async fn tags_route_requires_auth() {
    let response = request(
        test_app(empty_library()),
        Method::GET,
        "/tags",
        None,
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let payload = response_json(response).await;
    assert_eq!(payload["code"], "unauthorized");
}

#[tokio::test]
async fn tags_route_returns_empty_corpus() {
    let auth = bearer_token("user-sub");
    assert_token_decodes(&auth, "user-sub");

    let response = request(
        test_app(empty_library()),
        Method::GET,
        "/tags",
        Some(&auth),
        Body::Empty,
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response_json(response).await, serde_json::json!([]));
}

#[tokio::test]
async fn tag_route_renames_tag() {
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(tagged_library()),
        Method::PATCH,
        &format!("/tags/{}", source_tag_id()),
        Some(&auth),
        Body::from(r#"{"display_name":"Research"}"#),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload[0]["id"], source_tag_id().to_string());
    assert_eq!(payload[0]["display_name"], "Research");
    assert_eq!(payload[0]["normalized_name"], "research");
}

#[tokio::test]
async fn tag_route_merges_tags() {
    let auth = bearer_token("user-sub");
    let response = request(
        test_app(tagged_library()),
        Method::POST,
        &format!("/tags/{}/merge", source_tag_id()),
        Some(&auth),
        Body::from(format!(r#"{{"target_tag_id":"{}"}}"#, target_tag_id())),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload.as_array().unwrap().len(), 1);
    assert_eq!(payload[0]["id"], target_tag_id().to_string());
    assert_eq!(payload[0]["usage_count"], 2);
}

fn tagged_library() -> Arc<InMemoryLibraryService> {
    let service = InMemoryLibraryService::with_user_items(
        "user-sub",
        [
            item_with_tags(item_one_id(), vec![source_tag(), target_tag()]),
            item_with_tags(item_two_id(), vec![source_tag()]),
        ],
    );
    service.set_user_tags(
        "user-sub",
        [
            TagCorpusEntry {
                id: source_tag_id(),
                display_name: "Lerning".to_string(),
                normalized_name: "lerning".to_string(),
                usage_count: 2,
            },
            TagCorpusEntry {
                id: target_tag_id(),
                display_name: "Learning".to_string(),
                normalized_name: "learning".to_string(),
                usage_count: 1,
            },
        ],
    );
    Arc::new(service)
}

fn item_with_tags(item_id: Uuid, tags: Vec<ItemTag>) -> LibraryItemDetail {
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
            title: Some("Saved video".to_string()),
            fetched_title: None,
            thumbnail_s3_key: None,
            author: Some("Creator".to_string()),
            platform: Some("example".to_string()),
            duration_seconds: Some(42),
            archive_status: ArchiveStatus::Pending,
            watch_status: WatchStatus::Unwatched,
            inbox_status: InboxStatus::Unsorted,
            tags,
            created_at: OffsetDateTime::UNIX_EPOCH,
        },
        notes: String::new(),
    }
}

fn source_tag() -> ItemTag {
    ItemTag {
        id: source_tag_id(),
        display_name: "Lerning".to_string(),
        normalized_name: "lerning".to_string(),
    }
}

fn target_tag() -> ItemTag {
    ItemTag {
        id: target_tag_id(),
        display_name: "Learning".to_string(),
        normalized_name: "learning".to_string(),
    }
}

fn source_tag_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000301").unwrap()
}

fn target_tag_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000302").unwrap()
}

fn item_one_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000303").unwrap()
}

fn item_two_id() -> Uuid {
    Uuid::parse_str("00000000-0000-0000-0000-000000000304").unwrap()
}
