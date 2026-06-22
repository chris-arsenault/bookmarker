mod support;

use std::sync::Arc;

use api::image_access::{ImageObject, InMemoryImageObjectStore};
use axum::body::{to_bytes, Body};
use axum::http::{Method, StatusCode};
use shared::domain::{ArchiveStatus, ImageUploadStatus, InboxStatus, ItemKind, WatchStatus};
use shared::library::{
    InMemoryLibraryService, ItemImageSummary, LibraryItemDetail, LibraryItemSummary,
};
use time::OffsetDateTime;
use uuid::Uuid;

use support::{bearer_token, empty_library, request, response_json, test_app};

#[tokio::test]
async fn image_upload_route_creates_pending_image_and_upload_target() {
    let auth = bearer_token("image-user");
    let response = request(
        test_app(empty_library()),
        Method::POST,
        "/items/images/uploads",
        Some(&auth),
        Body::from(
            serde_json::json!({
                "content_type": "image/jpeg",
                "title": "Desk photo",
                "original_filename": "desk.jpg",
                "byte_size": 1234,
                "source_app": "Android share",
                "source_device": "android",
                "capture_method": "android_share",
                "tags": ["Photos"],
                "client_capture_id": "image-share-1"
            })
            .to_string(),
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::CREATED);
    let payload = response_json(response).await;
    assert_eq!(payload["created"], true);
    assert_eq!(payload["item"]["summary"]["item_kind"], "image");
    assert_eq!(payload["item"]["summary"]["archive_status"], "pending");
    assert_eq!(
        payload["item"]["summary"]["image"]["upload_status"],
        "pending"
    );
    assert_eq!(
        payload["item"]["summary"]["image"]["original_filename"],
        "desk.jpg"
    );
    assert!(payload["upload"]["url"]
        .as_str()
        .unwrap()
        .starts_with("https://upload.example.test/images/"));
    assert_eq!(payload["upload"]["headers"]["content-type"], "image/jpeg");
}

#[tokio::test]
async fn image_upload_complete_marks_image_uploaded() {
    let auth = bearer_token("image-user");
    let app = test_app(empty_library());
    let create = request(
        app.clone(),
        Method::POST,
        "/items/images/uploads",
        Some(&auth),
        Body::from(
            serde_json::json!({
                "content_type": "image/png",
                "client_capture_id": "image-share-complete"
            })
            .to_string(),
        ),
    )
    .await;
    let item_id = response_json(create).await["item"]["summary"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let response = request(
        app,
        Method::POST,
        &format!("/items/{item_id}/image-upload/complete"),
        Some(&auth),
        Body::empty(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let payload = response_json(response).await;
    assert_eq!(payload["summary"]["archive_status"], "succeeded");
    assert_eq!(payload["summary"]["image"]["upload_status"], "uploaded");
}

#[tokio::test]
async fn item_image_route_returns_owned_uploaded_image() {
    let item_id = item_id();
    let key = format!("images/{item_id}/original");
    let image_store = InMemoryImageObjectStore::from_objects([(
        key,
        ImageObject {
            bytes: b"image-bytes".to_vec(),
            content_type: "image/png".to_string(),
        },
    )]);
    let response = request(
        support::test_app_with_image_store(seeded_library(item_id), Arc::new(image_store)),
        Method::GET,
        &format!("/items/{item_id}/image"),
        Some(&bearer_token("image-user")),
        Body::empty(),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"].to_str().unwrap(),
        "image/png"
    );
    assert_eq!(
        to_bytes(response.into_body(), usize::MAX).await.unwrap(),
        b"image-bytes".as_slice()
    );
}

fn seeded_library(item_id: Uuid) -> Arc<InMemoryLibraryService> {
    Arc::new(InMemoryLibraryService::with_user_items(
        "image-user",
        [LibraryItemDetail {
            summary: LibraryItemSummary {
                id: item_id,
                item_kind: ItemKind::Image,
                url: None,
                text: None,
                image: Some(ItemImageSummary {
                    s3_key: format!("images/{item_id}/original"),
                    content_type: "image/png".to_string(),
                    original_filename: Some("phone.png".to_string()),
                    byte_size: Some(2048),
                    upload_status: ImageUploadStatus::Uploaded,
                    source_app: Some("Android share".to_string()),
                    source_device: Some("android".to_string()),
                    capture_method: "android_share".to_string(),
                }),
                title: Some("Phone image".to_string()),
                fetched_title: None,
                thumbnail_s3_key: None,
                author: None,
                platform: None,
                duration_seconds: None,
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
    Uuid::parse_str("00000000-0000-0000-0000-000000000401").unwrap()
}
