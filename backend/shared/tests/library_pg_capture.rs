use std::sync::Arc;

#[path = "support/sqlx.rs"]
mod sqlx_support;

use async_trait::async_trait;
use shared::auth::UserContext;
use shared::db::{
    LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION, LINKDROP_INBOX_STATUS_MIGRATION,
    LINKDROP_ITEM_TITLES_MIGRATION, LINKDROP_MODEL_MIGRATION, LINKDROP_TEXT_SNIPPET_MIGRATION,
};
use shared::domain::ArchiveStatus;
use shared::library::{CaptureItemOutcome, CaptureItemRequest, LibraryService, ListItemsQuery};
use shared::library_pg::PgLibraryService;
use shared::url_normalization::{ShortUrlResolveError, ShortUrlResolver};
use sqlx_support::{database_url, psql, setup_sqlx_postgres};

#[tokio::test]
async fn pg_capture_persists_original_and_canonical_url() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let service = PgLibraryService::new(pool.clone());

    let outcome = service
        .capture_item(
            &user(),
            CaptureItemRequest {
                url: "https://youtu.be/video-id?utm_source=share".to_string(),
                title: None,
                tags: Vec::new(),
                client_capture_id: Some("share-attempt-1".to_string()),
            },
        )
        .await
        .unwrap();

    assert!(outcome.created);
    assert_eq!(
        outcome.item.summary.url.as_ref().unwrap().original_url,
        "https://youtu.be/video-id?utm_source=share"
    );
    assert_eq!(
        outcome
            .item
            .summary
            .url
            .as_ref()
            .and_then(|url| url.canonical_url.as_deref()),
        Some("https://www.youtube.com/watch?v=video-id")
    );
    assert_eq!(
        outcome.item.summary.url.as_ref().unwrap().copy_url,
        "https://www.youtube.com/watch?v=video-id"
    );
    assert_eq!(outcome.item.summary.archive_status, ArchiveStatus::Pending);
    assert_eq!(outcome.item.summary.tags, Vec::new());
    assert_eq!(outcome.item.notes, "");

    let repeated = service
        .capture_item(
            &user(),
            CaptureItemRequest {
                url: "https://example.com/different".to_string(),
                title: None,
                tags: Vec::new(),
                client_capture_id: Some("share-attempt-1".to_string()),
            },
        )
        .await
        .unwrap();

    assert!(!repeated.created);
    assert_eq!(repeated.item.summary.id, outcome.item.summary.id);
    assert_eq!(
        repeated.item.summary.url.as_ref().unwrap().original_url,
        "https://youtu.be/video-id?utm_source=share"
    );
    assert_eq!(
        repeated
            .item
            .summary
            .url
            .as_ref()
            .and_then(|url| url.canonical_url.as_deref()),
        Some("https://www.youtube.com/watch?v=video-id")
    );
    assert_eq!(item_count(&service).await, 1);
    assert_eq!(count_rows(&pool, "metadata_snapshots").await, 0);
    assert_eq!(count_rows(&pool, "processing_jobs").await, 0);
}

#[tokio::test]
async fn pg_capture_applies_only_explicit_tags_and_updates_corpus() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let service = PgLibraryService::new(pool.clone());

    let first = service
        .capture_item(
            &user(),
            CaptureItemRequest {
                url: "https://example.com/one".to_string(),
                title: None,
                tags: vec![
                    " Learning ".to_string(),
                    "Videos".to_string(),
                    "learning".to_string(),
                ],
                client_capture_id: Some("share-attempt-tags-1".to_string()),
            },
        )
        .await
        .unwrap();

    assert_eq!(first.item.summary.tags.len(), 2);
    assert_eq!(first.item.summary.tags[0].normalized_name, "learning");
    assert_eq!(first.item.summary.tags[1].normalized_name, "videos");

    service
        .capture_item(
            &user(),
            CaptureItemRequest {
                url: "https://example.com/two".to_string(),
                title: None,
                tags: vec!["videos".to_string()],
                client_capture_id: Some("share-attempt-tags-2".to_string()),
            },
        )
        .await
        .unwrap();

    let corpus = service.list_tag_corpus(&user()).await.unwrap();
    assert_eq!(corpus.len(), 2);
    assert_eq!(corpus[0].display_name, "Videos");
    assert_eq!(corpus[0].usage_count, 2);
    assert_eq!(corpus[1].display_name, "Learning");
    assert_eq!(corpus[1].usage_count, 1);
    assert_eq!(non_explicit_tag_edges(&pool).await, 0);
}

#[tokio::test]
async fn pg_capture_text_persists_snippet_and_deduplicates_by_hash() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let service = PgLibraryService::new(pool);

    let first = service
        .capture_text(
            &user(),
            shared::library::CaptureTextRequest {
                plain_text: "clipboard note".to_string(),
                title: Some(" Terminal note ".to_string()),
                html: None,
                source_app: Some("Terminal".to_string()),
                source_device: None,
                capture_method: None,
                tags: vec!["Shell".to_string()],
                client_capture_id: Some("snippet-pg-1".to_string()),
            },
        )
        .await
        .unwrap();
    let second = service
        .capture_text(
            &user(),
            shared::library::CaptureTextRequest {
                client_capture_id: Some("snippet-pg-2".to_string()),
                tags: vec!["Later".to_string()],
                ..text_request("clipboard note")
            },
        )
        .await
        .unwrap();

    assert!(first.created);
    assert!(!second.created);
    assert_eq!(first.item.summary.id, second.item.summary.id);
    assert_eq!(
        first.item.summary.archive_status,
        ArchiveStatus::NotApplicable
    );
    assert_eq!(first.item.summary.title.as_deref(), Some("Terminal note"));
    assert_eq!(
        first
            .item
            .summary
            .text
            .as_ref()
            .unwrap()
            .source_app
            .as_deref(),
        Some("Terminal")
    );
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
async fn pg_capture_deduplicates_by_normalized_canonical_url() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let service =
        PgLibraryService::with_url_resolver(pool.clone(), Arc::new(FailingShortUrlResolver));

    let first = capture_url(
        &service,
        "https://youtu.be/video-id?utm_source=share&t=42",
        &["Learning"],
        "share-normalized-1",
    )
    .await;
    let second = capture_url(
        &service,
        "https://www.youtube.com/watch?v=video-id&t=42&utm_campaign=again",
        &["Later"],
        "share-normalized-2",
    )
    .await;

    assert_normalized_deduplication(&first, &second);
    assert_eq!(item_count(&service).await, 1);
    assert_single_learning_tag(&service).await;

    let failed_short = capture_url(
        &service,
        "https://vt.tiktok.com/ZSfailing/",
        &[],
        "share-short-failed",
    )
    .await;

    assert_failed_short_url_saved(&failed_short);
    assert_eq!(item_count(&service).await, 2);
}

async fn capture_url(
    service: &PgLibraryService,
    url: &str,
    tags: &[&str],
    client_capture_id: &str,
) -> CaptureItemOutcome {
    service
        .capture_item(
            &user(),
            CaptureItemRequest {
                url: url.to_string(),
                title: None,
                tags: tags.iter().map(|tag| (*tag).to_string()).collect(),
                client_capture_id: Some(client_capture_id.to_string()),
            },
        )
        .await
        .unwrap()
}

fn assert_normalized_deduplication(first: &CaptureItemOutcome, second: &CaptureItemOutcome) {
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
}

async fn item_count(service: &PgLibraryService) -> usize {
    service
        .list_items(&user(), &ListItemsQuery::default())
        .await
        .unwrap()
        .len()
}

async fn assert_single_learning_tag(service: &PgLibraryService) {
    let corpus = service.list_tag_corpus(&user()).await.unwrap();
    assert_eq!(corpus.len(), 1);
    assert_eq!(corpus[0].normalized_name, "learning");
}

fn assert_failed_short_url_saved(failed_short: &CaptureItemOutcome) {
    assert!(failed_short.created);
    assert_eq!(
        failed_short
            .item
            .summary
            .url
            .as_ref()
            .unwrap()
            .canonical_url,
        None
    );
    assert_eq!(
        failed_short.item.summary.url.as_ref().unwrap().copy_url,
        "https://vt.tiktok.com/ZSfailing/"
    );
}

fn text_request(plain_text: &str) -> shared::library::CaptureTextRequest {
    shared::library::CaptureTextRequest {
        plain_text: plain_text.to_string(),
        title: None,
        html: None,
        source_app: None,
        source_device: None,
        capture_method: None,
        tags: Vec::new(),
        client_capture_id: None,
    }
}

struct FailingShortUrlResolver;

#[async_trait]
impl ShortUrlResolver for FailingShortUrlResolver {
    async fn resolve(&self, _url: &str) -> Result<String, ShortUrlResolveError> {
        Err(ShortUrlResolveError::new("resolver unavailable"))
    }
}

fn user() -> UserContext {
    UserContext {
        sub: "capture-user".to_string(),
        email: Some("capture@example.test".to_string()),
        username: Some("capture-user".to_string()),
        groups: vec![],
    }
}

fn apply_migrations(container_name: &str) {
    run_psql(container_name, LINKDROP_MODEL_MIGRATION);
    run_psql(container_name, LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION);
    run_psql(container_name, LINKDROP_INBOX_STATUS_MIGRATION);
    run_psql(container_name, LINKDROP_TEXT_SNIPPET_MIGRATION);
    run_psql(container_name, LINKDROP_ITEM_TITLES_MIGRATION);
}

fn run_psql(container_name: &str, sql: &str) {
    let output = psql(container_name, sql);
    assert!(
        output.status.success(),
        "psql failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

async fn count_rows(pool: &sqlx::PgPool, table: &str) -> i64 {
    let query = format!("SELECT count(*) FROM {table}");
    sqlx::query_scalar(&query).fetch_one(pool).await.unwrap()
}

async fn non_explicit_tag_edges(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM item_tags WHERE applied_source <> 'explicit'")
        .fetch_one(pool)
        .await
        .unwrap()
}
