#[path = "support/sqlx.rs"]
mod sqlx_support;

use shared::auth::UserContext;
use shared::db::LINKDROP_MODEL_MIGRATION;
use shared::library::{LibraryService, MergeTagsRequest};
use shared::library_pg::PgLibraryService;
use sqlx_support::{database_url, psql, setup_sqlx_postgres};
use uuid::Uuid;

#[tokio::test]
async fn pg_merge_tags_preserves_associations_and_usage_counts() {
    let container = setup_sqlx_postgres();
    apply_migration(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let user_id = seed_user(&pool, "filter-user").await;
    let source_tag_id = seed_tag(&pool, user_id, "Lerning").await;
    let target_tag_id = seed_tag(&pool, user_id, "Learning").await;
    let item_with_both = seed_item(&pool, user_id).await;
    let item_source_only = seed_item(&pool, user_id).await;
    link_item_tag(&pool, item_with_both, user_id, source_tag_id).await;
    link_item_tag(&pool, item_with_both, user_id, target_tag_id).await;
    link_item_tag(&pool, item_source_only, user_id, source_tag_id).await;

    let service = PgLibraryService::new(pool.clone());
    let corpus = service
        .merge_tags(&user(), source_tag_id, MergeTagsRequest { target_tag_id })
        .await
        .unwrap();

    assert_eq!(corpus.len(), 1);
    assert_eq!(corpus[0].id, target_tag_id);
    assert_eq!(corpus[0].usage_count, 2);
    assert_eq!(edge_count(&pool, source_tag_id).await, 0);
    assert_eq!(edge_count(&pool, target_tag_id).await, 2);
}

async fn seed_user(pool: &sqlx::PgPool, sub: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO users (cognito_sub) VALUES ($1) RETURNING id")
        .bind(sub)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_item(pool: &sqlx::PgPool, user_id: Uuid) -> Uuid {
    sqlx::query_scalar("INSERT INTO items (user_id) VALUES ($1) RETURNING id")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn seed_tag(pool: &sqlx::PgPool, user_id: Uuid, display_name: &str) -> Uuid {
    sqlx::query_scalar("INSERT INTO tags (user_id, display_name) VALUES ($1, $2) RETURNING id")
        .bind(user_id)
        .bind(display_name)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn link_item_tag(pool: &sqlx::PgPool, item_id: Uuid, user_id: Uuid, tag_id: Uuid) {
    sqlx::query("INSERT INTO item_tags (item_id, tag_id, user_id) VALUES ($1, $2, $3)")
        .bind(item_id)
        .bind(tag_id)
        .bind(user_id)
        .execute(pool)
        .await
        .unwrap();
}

async fn edge_count(pool: &sqlx::PgPool, tag_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM item_tags WHERE tag_id = $1")
        .bind(tag_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn user() -> UserContext {
    UserContext {
        sub: "filter-user".to_string(),
        email: Some("filter@example.test".to_string()),
        username: Some("filter-user".to_string()),
        groups: vec![],
    }
}

fn apply_migration(container_name: &str) {
    let output = psql(container_name, LINKDROP_MODEL_MIGRATION);
    assert!(
        output.status.success(),
        "psql failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
