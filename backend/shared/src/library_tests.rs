use time::OffsetDateTime;
use uuid::Uuid;

use super::*;

fn user() -> UserContext {
    UserContext {
        sub: "user-sub".to_string(),
        email: Some("chris@example.test".to_string()),
        username: Some("chris".to_string()),
        groups: vec![],
    }
}

fn item(id: Uuid) -> LibraryItemDetail {
    LibraryItemDetail {
        summary: LibraryItemSummary {
            id,
            item_kind: ItemKind::Url,
            url: Some(ItemUrlSummary::new(
                "https://example.com/watch".to_string(),
                None,
            )),
            text: None,
            image: None,
            title: Some("Example".to_string()),
            fetched_title: None,
            thumbnail_s3_key: None,
            author: None,
            platform: None,
            duration_seconds: None,
            archive_status: ArchiveStatus::Pending,
            watch_status: WatchStatus::Unwatched,
            inbox_status: InboxStatus::Unsorted,
            tags: vec![],
            created_at: OffsetDateTime::UNIX_EPOCH,
        },
        notes: String::new(),
    }
}

struct FilteredItem {
    title: &'static str,
    notes: &'static str,
    platform: &'static str,
    archive_status: ArchiveStatus,
    watch_status: WatchStatus,
    tag_name: &'static str,
    created_at: i64,
}

impl FilteredItem {
    fn matching() -> Self {
        Self {
            title: "Rust async talk",
            notes: "Rewatch for metadata pipeline notes",
            platform: "YouTube",
            archive_status: ArchiveStatus::Succeeded,
            watch_status: WatchStatus::Unwatched,
            tag_name: "learning",
            created_at: 1_700_000_000,
        }
    }
}

fn filtered_item(id: Uuid, fixture: FilteredItem) -> LibraryItemDetail {
    LibraryItemDetail {
        summary: LibraryItemSummary {
            id,
            title: Some(fixture.title.to_string()),
            platform: Some(fixture.platform.to_string()),
            archive_status: fixture.archive_status,
            watch_status: fixture.watch_status,
            tags: vec![test_tag(fixture.tag_name)],
            created_at: timestamp(fixture.created_at),
            ..item(id).summary
        },
        notes: fixture.notes.to_string(),
    }
}

fn test_tag(name: &str) -> ItemTag {
    ItemTag {
        id: Uuid::new_v4(),
        display_name: name.to_string(),
        normalized_name: name.to_ascii_lowercase(),
    }
}

fn timestamp(value: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(value).unwrap()
}

#[test]
fn item_summary_copy_url_prefers_canonical_url() {
    assert_eq!(
        ItemUrlSummary::copy_url_for(
            "https://example.com/watch?utm_source=share",
            Some("https://example.com/watch"),
        ),
        "https://example.com/watch"
    );
    assert_eq!(
        ItemUrlSummary::copy_url_for("https://example.com/watch?utm_source=share", None),
        "https://example.com/watch?utm_source=share"
    );
}

#[tokio::test]
async fn in_memory_library_returns_empty_state_for_new_user() {
    let service = InMemoryLibraryService::new();

    assert_eq!(
        service
            .list_items(&user(), &ListItemsQuery::default())
            .await
            .unwrap(),
        Vec::new()
    );
    assert_eq!(service.list_tag_corpus(&user()).await.unwrap(), Vec::new());
}

#[tokio::test]
async fn in_memory_list_filters_items_by_status_tag_date_and_text() {
    let matching_id = Uuid::parse_str("00000000-0000-0000-0000-000000000601").unwrap();
    let hidden_id = Uuid::parse_str("00000000-0000-0000-0000-000000000602").unwrap();
    let service = InMemoryLibraryService::with_user_items(
        "user-sub",
        [
            filtered_item(matching_id, FilteredItem::matching()),
            filtered_item(
                hidden_id,
                FilteredItem {
                    platform: "TikTok",
                    ..FilteredItem::matching()
                },
            ),
        ],
    );

    let items = service
        .list_items(
            &user(),
            &ListItemsQuery {
                platform: Some("youtube".to_string()),
                tag: Some("learning".to_string()),
                created_from: Some(timestamp(1_699_999_999)),
                created_to: Some(timestamp(1_700_000_001)),
                archive_status: Some(ArchiveStatus::Succeeded),
                watch_status: Some(WatchStatus::Unwatched),
                inbox_status: None,
                q: Some("pipeline".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(
        items.into_iter().map(|item| item.id).collect::<Vec<_>>(),
        vec![matching_id]
    );
}

#[tokio::test]
async fn in_memory_library_reads_and_updates_existing_item_watch_status() {
    let item_id = Uuid::new_v4();
    let service = InMemoryLibraryService::with_user_items("user-sub", [item(item_id)]);

    let detail = service.get_item(&user(), item_id).await.unwrap();
    assert_eq!(detail.summary.watch_status, WatchStatus::Unwatched);

    let updated = service
        .update_item(
            &user(),
            item_id,
            UpdateItemRequest {
                watch_status: Some(WatchStatus::Watched),
                ..UpdateItemRequest::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.summary.watch_status, WatchStatus::Watched);
}

#[tokio::test]
async fn in_memory_capture_accepts_url_without_tags() {
    let service = InMemoryLibraryService::new();

    let outcome = service
        .capture_item(
            &user(),
            CaptureItemRequest {
                url: "https://example.com/watch?utm_source=share".to_string(),
                title: None,
                tags: Vec::new(),
                client_capture_id: None,
            },
        )
        .await
        .unwrap();

    assert!(outcome.created);
    assert_eq!(
        outcome.item.summary.url.as_ref().unwrap().original_url,
        "https://example.com/watch?utm_source=share"
    );
    assert_eq!(
        outcome
            .item
            .summary
            .url
            .as_ref()
            .and_then(|url| url.canonical_url.as_deref()),
        Some("https://example.com/watch")
    );
    assert_eq!(
        outcome.item.summary.url.as_ref().unwrap().copy_url,
        "https://example.com/watch"
    );
    assert_eq!(outcome.item.summary.archive_status, ArchiveStatus::Pending);
    assert_eq!(outcome.item.summary.tags, Vec::new());
}

#[tokio::test]
async fn in_memory_capture_accepts_optional_link_title() {
    let service = InMemoryLibraryService::new();

    let outcome = service
        .capture_item(
            &user(),
            CaptureItemRequest {
                url: "https://example.com/watch".to_string(),
                title: Some(" Shared title ".to_string()),
                tags: Vec::new(),
                client_capture_id: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(outcome.item.summary.title.as_deref(), Some("Shared title"));
}

#[tokio::test]
async fn in_memory_capture_accepts_text_snippet() {
    let service = InMemoryLibraryService::new();

    let outcome = service
        .capture_text(
            &user(),
            CaptureTextRequest {
                plain_text: " Keep this snippet nearby ".to_string(),
                title: Some(" Snippet title ".to_string()),
                html: None,
                source_app: Some("Editor".to_string()),
                source_device: Some("laptop".to_string()),
                capture_method: None,
                tags: vec!["Drafts".to_string()],
                client_capture_id: Some("snippet-1".to_string()),
            },
        )
        .await
        .unwrap();

    assert!(outcome.created);
    assert_eq!(outcome.item.summary.item_kind, ItemKind::TextSnippet);
    assert_eq!(outcome.item.summary.title.as_deref(), Some("Snippet title"));
    assert_eq!(
        outcome.item.summary.text.as_ref().unwrap().plain_text,
        " Keep this snippet nearby "
    );
    assert_eq!(
        outcome.item.summary.archive_status,
        ArchiveStatus::NotApplicable
    );
    assert_eq!(outcome.item.summary.tags[0].normalized_name, "drafts");
}

#[tokio::test]
async fn in_memory_capture_applies_only_explicit_tags() {
    let service = InMemoryLibraryService::new();

    let outcome = service
        .capture_item(
            &user(),
            CaptureItemRequest {
                url: "https://example.com/watch".to_string(),
                title: None,
                tags: vec![" Learning ".to_string(), "learning".to_string()],
                client_capture_id: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(outcome.item.summary.tags.len(), 1);
    assert_eq!(outcome.item.summary.tags[0].display_name, "Learning");
    assert_eq!(outcome.item.summary.tags[0].normalized_name, "learning");
    assert_eq!(
        service.list_tag_corpus(&user()).await.unwrap()[0].usage_count,
        1
    );
}

#[tokio::test]
async fn in_memory_capture_reuses_client_capture_id() {
    let service = InMemoryLibraryService::new();
    let request = CaptureItemRequest {
        url: "https://example.com/watch".to_string(),
        title: None,
        tags: Vec::new(),
        client_capture_id: Some("share-attempt-1".to_string()),
    };

    let first = service
        .capture_item(&user(), request.clone())
        .await
        .unwrap();
    let second = service.capture_item(&user(), request).await.unwrap();

    assert!(first.created);
    assert!(!second.created);
    assert_eq!(first.item.summary.id, second.item.summary.id);
    assert_eq!(
        service
            .list_items(&user(), &ListItemsQuery::default())
            .await
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn in_memory_capture_deduplicates_by_normalized_url() {
    let service = InMemoryLibraryService::new();

    let first = service
        .capture_item(
            &user(),
            CaptureItemRequest {
                url: "https://youtu.be/video-id?utm_source=share&t=42".to_string(),
                title: None,
                tags: vec!["Learning".to_string()],
                client_capture_id: Some("share-attempt-normalized-1".to_string()),
            },
        )
        .await
        .unwrap();
    let second = service
        .capture_item(
            &user(),
            CaptureItemRequest {
                url: "https://www.youtube.com/watch?v=video-id&t=42&utm_campaign=again".to_string(),
                title: None,
                tags: vec!["Later".to_string()],
                client_capture_id: Some("share-attempt-normalized-2".to_string()),
            },
        )
        .await
        .unwrap();

    assert!(first.created);
    assert!(!second.created);
    assert_eq!(first.item.summary.id, second.item.summary.id);
    assert_eq!(
        first
            .item
            .summary
            .url
            .as_ref()
            .and_then(|url| url.canonical_url.as_deref()),
        Some("https://www.youtube.com/watch?v=video-id&t=42")
    );
    assert_eq!(
        first.item.summary.url.as_ref().unwrap().copy_url,
        "https://www.youtube.com/watch?v=video-id&t=42"
    );
    assert_eq!(
        service
            .list_items(&user(), &ListItemsQuery::default())
            .await
            .unwrap()
            .len(),
        1
    );

    let corpus = service.list_tag_corpus(&user()).await.unwrap();
    assert_eq!(corpus.len(), 1);
    assert_eq!(corpus[0].normalized_name, "learning");
}

#[path = "library_tests_m7.rs"]
mod m7;
