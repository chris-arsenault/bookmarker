#[path = "support/sqlx.rs"]
mod sqlx_support;

use shared::auth::UserContext;
use shared::db::{
    LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION, LINKDROP_INBOX_STATUS_MIGRATION,
    LINKDROP_ITEM_DELETIONS_MIGRATION, LINKDROP_MODEL_MIGRATION, LINKDROP_TEXT_SNIPPET_MIGRATION,
};
use shared::library::{LibraryService, ListItemUpdatesQuery, ListItemsQuery};
use shared::library_pg::PgLibraryService;
use sqlx_support::{database_url, psql, setup_sqlx_postgres};
use time::OffsetDateTime;
use uuid::Uuid;

#[tokio::test]
async fn pg_item_updates_include_metadata_snapshot_changes() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let user_id = seed_user(&pool).await;
    let item_id = seed_link_item(&pool, user_id).await;
    let service = PgLibraryService::new(pool);

    let updates = service
        .list_item_updates(
            &user(),
            &ListItemUpdatesQuery {
                since: Some(timestamp(150)),
                limit: 10,
                filters: ListItemsQuery::default(),
            },
        )
        .await
        .unwrap();

    assert_eq!(updates.items.len(), 1);
    assert_eq!(updates.items[0].id, item_id);
    assert_eq!(updates.items[0].title.as_deref(), Some("Updated title"));
    assert!(updates.deleted_item_ids.is_empty());
}

#[tokio::test]
async fn pg_item_updates_include_deleted_item_ids() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let user_id = seed_user(&pool).await;
    let item_id = seed_link_item(&pool, user_id).await;
    let service = PgLibraryService::new(pool);
    let since = OffsetDateTime::now_utc() - time::Duration::seconds(1);

    service.delete_item(&user(), item_id).await.unwrap();
    let updates = service
        .list_item_updates(
            &user(),
            &ListItemUpdatesQuery {
                since: Some(since),
                limit: 10,
                filters: ListItemsQuery::default(),
            },
        )
        .await
        .unwrap();

    assert!(updates.items.is_empty());
    assert_eq!(updates.deleted_item_ids, vec![item_id]);
}

async fn seed_user(pool: &sqlx::PgPool) -> Uuid {
    sqlx::query_scalar("INSERT INTO users (cognito_sub) VALUES ('updates-user') RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_link_item(pool: &sqlx::PgPool, user_id: Uuid) -> Uuid {
    let item_id: Uuid = sqlx::query_scalar(
        "INSERT INTO items (user_id, created_at, updated_at)
         VALUES ($1, $2, $2) RETURNING id",
    )
    .bind(user_id)
    .bind(timestamp(100))
    .fetch_one(pool)
    .await
    .unwrap();
    insert_url(pool, item_id, user_id).await;
    insert_snapshot(pool, item_id, user_id).await;
    item_id
}

async fn insert_url(pool: &sqlx::PgPool, item_id: Uuid, user_id: Uuid) {
    sqlx::query("INSERT INTO item_urls (item_id, user_id, original_url) VALUES ($1, $2, $3)")
        .bind(item_id)
        .bind(user_id)
        .bind("https://example.com/updates")
        .execute(pool)
        .await
        .unwrap();
}

async fn insert_snapshot(pool: &sqlx::PgPool, item_id: Uuid, user_id: Uuid) {
    sqlx::query(
        "INSERT INTO metadata_snapshots (item_id, user_id, title, updated_at)
         VALUES ($1, $2, 'Updated title', $3)",
    )
    .bind(item_id)
    .bind(user_id)
    .bind(timestamp(200))
    .execute(pool)
    .await
    .unwrap();
}

fn user() -> UserContext {
    UserContext {
        sub: "updates-user".to_string(),
        email: None,
        username: None,
        groups: vec![],
    }
}

fn apply_migrations(container_name: &str) {
    run_psql(container_name, LINKDROP_MODEL_MIGRATION);
    run_psql(container_name, LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION);
    run_psql(container_name, LINKDROP_INBOX_STATUS_MIGRATION);
    run_psql(container_name, LINKDROP_TEXT_SNIPPET_MIGRATION);
    run_psql(container_name, LINKDROP_ITEM_DELETIONS_MIGRATION);
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

fn timestamp(seconds: i64) -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(seconds).unwrap()
}
