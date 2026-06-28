mod support;

use std::sync::Arc;

use api::thumbnail_access::{InMemoryThumbnailReader, ThumbnailObject};
use lambda_http::http::{Method, StatusCode};
use lambda_http::Body;
use shared::domain::{ArchiveStatus, InboxStatus, ItemKind, WatchStatus};
use shared::library::{
    InMemoryLibraryService, ItemUrlSummary, LibraryItemDetail, LibraryItemSummary,
};
use time::OffsetDateTime;
use uuid::Uuid;

use support::{bearer_token, request, test_app_with_thumbnail_reader};

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
        Body::Empty,
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
        api::http_body_bytes(response.body()),
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
