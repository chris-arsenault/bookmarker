#[path = "support/sqlx.rs"]
mod sqlx_support;

use shared::auth::UserContext;
use shared::db::{
    LINKDROP_IMAGE_ITEMS_MIGRATION, LINKDROP_INBOX_STATUS_MIGRATION,
    LINKDROP_ITEM_TITLES_MIGRATION, LINKDROP_MODEL_MIGRATION, LINKDROP_TEXT_SNIPPET_MIGRATION,
};
use shared::domain::{ArchiveStatus, InboxStatus, WatchStatus};
use shared::library::{LibraryService, ListItemsQuery, UpdateItemRequest};
use shared::library_pg::PgLibraryService;
use sqlx_support::{database_url, psql, setup_sqlx_postgres};
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn pg_list_items_filters_by_metadata_tag_status_and_notes() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let user_id = seed_user(&pool, "filter-user").await;
    let other_user_id = seed_user(&pool, "other-user").await;
    let matching = seed_item(
        &pool,
        user_id,
        SeedItem::matching("https://example.com/matching"),
    )
    .await;
    seed_item(
        &pool,
        user_id,
        SeedItem {
            platform: "TikTok",
            ..SeedItem::matching("https://example.com/hidden")
        },
    )
    .await;
    seed_item(
        &pool,
        other_user_id,
        SeedItem::matching("https://example.com/other-user"),
    )
    .await;

    let service = PgLibraryService::new(pool);
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
        vec![matching]
    );
}

#[tokio::test]
async fn pg_list_items_filters_by_inbox_status() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let user_id = seed_user(&pool, "filter-user").await;
    let matching = seed_item(
        &pool,
        user_id,
        SeedItem {
            inbox_status: InboxStatus::Organized,
            ..SeedItem::matching("https://example.com/organized")
        },
    )
    .await;
    seed_item(
        &pool,
        user_id,
        SeedItem::matching("https://example.com/unsorted"),
    )
    .await;

    let service = PgLibraryService::new(pool);
    let items = service
        .list_items(
            &user(),
            &ListItemsQuery {
                inbox_status: Some(InboxStatus::Organized),
                ..ListItemsQuery::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(
        items.into_iter().map(|item| item.id).collect::<Vec<_>>(),
        vec![matching]
    );
}

#[tokio::test]
async fn pg_update_item_replaces_tags_notes_watch_and_inbox() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let user_id = seed_user(&pool, "filter-user").await;
    let item_id = seed_item(
        &pool,
        user_id,
        SeedItem::matching("https://example.com/organize"),
    )
    .await;
    let service = PgLibraryService::new(pool);

    let updated = service
        .update_item(
            &user(),
            item_id,
            UpdateItemRequest {
                title: None,
                watch_status: Some(WatchStatus::Watched),
                inbox_status: Some(InboxStatus::Organized),
                notes: Some("Filed after watching".to_string()),
                tags: Some(vec![" Videos ".to_string(), "Later".to_string()]),
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.summary.watch_status, WatchStatus::Watched);
    assert_eq!(updated.summary.inbox_status, InboxStatus::Organized);
    assert_eq!(updated.notes, "Filed after watching");
    assert_eq!(
        updated
            .summary
            .tags
            .iter()
            .map(|tag| tag.normalized_name.as_str())
            .collect::<Vec<_>>(),
        vec!["later", "videos"]
    );
    assert_eq!(
        service
            .list_tag_corpus(&user())
            .await
            .unwrap()
            .iter()
            .map(|tag| tag.normalized_name.as_str())
            .collect::<Vec<_>>(),
        vec!["later", "videos"]
    );
}

#[tokio::test]
async fn pg_update_item_edits_and_clears_item_title() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let user_id = seed_user(&pool, "filter-user").await;
    let item_id = seed_item(
        &pool,
        user_id,
        SeedItem::matching("https://example.com/title-edit"),
    )
    .await;
    let service = PgLibraryService::new(pool);

    let renamed = service
        .update_item(
            &user(),
            item_id,
            UpdateItemRequest {
                title: Some(" Renamed item ".to_string()),
                ..UpdateItemRequest::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(renamed.summary.title.as_deref(), Some("Renamed item"));
    assert_eq!(
        renamed.summary.fetched_title.as_deref(),
        Some("Rust async talk")
    );

    let cleared = service
        .update_item(
            &user(),
            item_id,
            UpdateItemRequest {
                title: Some("   ".to_string()),
                ..UpdateItemRequest::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(cleared.summary.title, None);
    assert_eq!(
        cleared.summary.fetched_title.as_deref(),
        Some("Rust async talk")
    );
}

struct SeedItem {
    original_url: String,
    title: &'static str,
    platform: &'static str,
    notes: &'static str,
    tag: &'static str,
    archive_status: ArchiveStatus,
    watch_status: WatchStatus,
    inbox_status: InboxStatus,
    created_at: i64,
}

impl SeedItem {
    fn matching(original_url: &str) -> Self {
        Self {
            original_url: original_url.to_string(),
            title: "Rust async talk",
            platform: "YouTube",
            notes: "Rewatch for metadata pipeline notes",
            tag: "Learning",
            archive_status: ArchiveStatus::Succeeded,
            watch_status: WatchStatus::Unwatched,
            inbox_status: InboxStatus::Unsorted,
            created_at: 1_700_000_000,
        }
    }
}

async fn seed_user(pool: &sqlx::PgPool, sub: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO users (cognito_sub) VALUES ($1) RETURNING id")
        .bind(sub)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_item(pool: &sqlx::PgPool, user_id: Uuid, item: SeedItem) -> Uuid {
    let item_id: Uuid = sqlx::query_scalar(
        "INSERT INTO items (user_id, watch_status, inbox_status, created_at)
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(user_id)
    .bind(item.watch_status.as_str())
    .bind(item.inbox_status.as_str())
    .bind(timestamp(item.created_at))
    .fetch_one(pool)
    .await
    .unwrap();
    insert_url(pool, item_id, user_id, &item.original_url).await;
    insert_snapshot(pool, item_id, user_id, &item).await;
    insert_note(pool, item_id, user_id, item.notes).await;
    insert_tag(pool, item_id, user_id, item.tag).await;
    item_id
}

async fn insert_url(pool: &sqlx::PgPool, item_id: Uuid, user_id: Uuid, original_url: &str) {
    sqlx::query("INSERT INTO item_urls (item_id, user_id, original_url) VALUES ($1, $2, $3)")
        .bind(item_id)
        .bind(user_id)
        .bind(original_url)
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_snapshot(pool: &sqlx::PgPool, item_id: Uuid, user_id: Uuid, item: &SeedItem) {
    sqlx::query(
        "INSERT INTO metadata_snapshots (
            item_id, user_id, title, platform, archive_status
         )
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(item_id)
    .bind(user_id)
    .bind(item.title)
    .bind(item.platform)
    .bind(item.archive_status.as_str())
    .execute(pool)
    .await
    .unwrap();
}

async fn insert_note(pool: &sqlx::PgPool, item_id: Uuid, user_id: Uuid, notes: &str) {
    sqlx::query("INSERT INTO item_notes (item_id, user_id, body) VALUES ($1, $2, $3)")
        .bind(item_id)
        .bind(user_id)
        .bind(notes)
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_tag(pool: &sqlx::PgPool, item_id: Uuid, user_id: Uuid, display_name: &str) {
    let tag_id: Uuid = sqlx::query_scalar(
        "INSERT INTO tags (user_id, display_name)
         VALUES ($1, $2)
         ON CONFLICT (user_id, normalized_name)
         DO UPDATE SET updated_at = tags.updated_at
         RETURNING id",
    )
    .bind(user_id)
    .bind(display_name)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO item_tags (item_id, tag_id, user_id) VALUES ($1, $2, $3)")
        .bind(item_id)
        .bind(tag_id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
}

fn user() -> UserContext {
    UserContext {
        sub: "filter-user".to_string(),
        email: Some("filter@example.test".to_string()),
        username: Some("filter-user".to_string()),
        groups: vec![],
    }
}

fn timestamp(value: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(value).unwrap()
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

fn apply_migrations(container_name: &str) {
    run_psql(container_name, LINKDROP_MODEL_MIGRATION);
    run_psql(container_name, LINKDROP_INBOX_STATUS_MIGRATION);
    run_psql(container_name, LINKDROP_TEXT_SNIPPET_MIGRATION);
    run_psql(container_name, LINKDROP_ITEM_TITLES_MIGRATION);
    run_psql(container_name, LINKDROP_IMAGE_ITEMS_MIGRATION);
}
