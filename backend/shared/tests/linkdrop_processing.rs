#[path = "support/sqlx.rs"]
mod sqlx_support;
mod support;

use shared::db::LINKDROP_MODEL_MIGRATION;
use shared::domain::{ArchiveStatus, ProcessingJobKind, ProcessingStatus};
use shared::processing::{ProcessingRepository, SnapshotUpdate};
use sqlx_support::{database_url, setup_sqlx_postgres};
use support::{psql, setup_postgres};
use uuid::Uuid;

#[test]
fn linkdrop_model_supports_idempotent_archive_and_processing_status_updates() {
    let container = setup_postgres();
    run_psql(&container.name, LINKDROP_MODEL_MIGRATION);

    let user_id = insert_user(&container.name, "processing-user");
    let item_id = insert_item(&container.name, &user_id);

    upsert_snapshot(&container.name, &item_id, &user_id, "pending");
    upsert_snapshot(&container.name, &item_id, &user_id, "failed");
    upsert_snapshot(&container.name, &item_id, &user_id, "succeeded");
    assert_eq!(row_count(&container.name, "metadata_snapshots"), 1);
    assert_eq!(snapshot_status(&container.name, &item_id), "succeeded");
    assert_psql_fails(
        &container.name,
        &snapshot_upsert_sql(&item_id, &user_id, "archived"),
    );

    upsert_job(&container.name, &item_id, &user_id, "queued", 0);
    upsert_job(&container.name, &item_id, &user_id, "running", 1);
    upsert_job(&container.name, &item_id, &user_id, "succeeded", 2);
    assert_eq!(row_count(&container.name, "processing_jobs"), 1);
    assert_eq!(job_attempt_count(&container.name, &item_id), 2);

    assert_psql_fails(
        &container.name,
        &job_insert_sql(&item_id, &user_id, "normalize_url", "pending", 0),
    );
    assert_psql_fails(
        &container.name,
        &job_insert_sql(&item_id, &user_id, "snapshot_thumbnail", "queued", -1),
    );
}

#[tokio::test]
async fn processing_repository_queues_and_retries_enrichment_jobs() {
    let container = setup_sqlx_postgres();
    run_psql(&container.name, LINKDROP_MODEL_MIGRATION);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let item_id = seed_processing_item(&pool).await;
    let repository = ProcessingRepository::new(pool.clone());

    let queued = repository
        .enqueue_job(item_id, ProcessingJobKind::EnrichMetadata)
        .await
        .unwrap();
    repository
        .enqueue_job(item_id, ProcessingJobKind::EnrichMetadata)
        .await
        .unwrap();

    assert_eq!(queued.status, ProcessingStatus::Queued);
    assert_eq!(processing_job_count(&pool).await, 1);

    let item = repository.load_item(item_id).await.unwrap();
    assert_eq!(item.source_url, "https://example.com/watch?v=1");

    let running = repository
        .mark_job_running(item_id, ProcessingJobKind::EnrichMetadata, "worker-1")
        .await
        .unwrap();
    assert_eq!(running.status, ProcessingStatus::Running);
    assert_eq!(running.attempt_count, 1);

    repository
        .mark_job_failed(item_id, ProcessingJobKind::EnrichMetadata, "blocked")
        .await
        .unwrap();
    repository
        .enqueue_job(item_id, ProcessingJobKind::EnrichMetadata)
        .await
        .unwrap();
    assert_eq!(job_status(&pool, item_id).await, "queued");

    repository
        .upsert_snapshot(
            item_id,
            SnapshotUpdate {
                title: Some("Saved title".to_string()),
                thumbnail_s3_key: Some("snapshots/item/thumbnail.jpg".to_string()),
                thumbnail_content_type: Some("image/jpeg".to_string()),
                author: Some("Creator".to_string()),
                platform: Some("example".to_string()),
                duration_seconds: Some(42),
                archive_status: ArchiveStatus::Succeeded,
                archive_error: None,
            },
        )
        .await
        .unwrap();
    repository
        .upsert_snapshot(
            item_id,
            SnapshotUpdate {
                archive_status: ArchiveStatus::Failed,
                archive_error: Some("source blocked".to_string()),
                ..SnapshotUpdate::default()
            },
        )
        .await
        .unwrap();

    assert_eq!(row_count(&container.name, "metadata_snapshots"), 1);
    assert_eq!(
        snapshot_status(&container.name, &item_id.to_string()),
        "failed"
    );
    assert_eq!(
        snapshot_thumbnail_key(&pool, item_id).await.as_deref(),
        Some("snapshots/item/thumbnail.jpg")
    );
}

fn insert_user(container_name: &str, cognito_sub: &str) -> String {
    query_value(
        container_name,
        &format!("INSERT INTO users (cognito_sub) VALUES ('{cognito_sub}') RETURNING id;"),
    )
}

async fn seed_processing_item(pool: &sqlx::PgPool) -> Uuid {
    let user_id: Uuid = sqlx::query_scalar(
        "INSERT INTO users (cognito_sub) VALUES ('processing-repository-user') RETURNING id",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let item_id: Uuid = sqlx::query_scalar("INSERT INTO items (user_id) VALUES ($1) RETURNING id")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO item_urls (item_id, user_id, original_url, canonical_url)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(item_id)
    .bind(user_id)
    .bind("https://example.com/original?utm_source=share")
    .bind("https://example.com/watch?v=1")
    .execute(pool)
    .await
    .unwrap();
    item_id
}

async fn processing_job_count(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM processing_jobs")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn job_status(pool: &sqlx::PgPool, item_id: Uuid) -> String {
    sqlx::query_scalar("SELECT status FROM processing_jobs WHERE item_id = $1")
        .bind(item_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn snapshot_thumbnail_key(pool: &sqlx::PgPool, item_id: Uuid) -> Option<String> {
    sqlx::query_scalar("SELECT thumbnail_s3_key FROM metadata_snapshots WHERE item_id = $1")
        .bind(item_id)
        .fetch_one(pool)
        .await
        .unwrap()
}

fn insert_item(container_name: &str, user_id: &str) -> String {
    query_value(
        container_name,
        &format!("INSERT INTO items (user_id) VALUES ('{user_id}') RETURNING id;"),
    )
}

fn upsert_snapshot(container_name: &str, item_id: &str, user_id: &str, status: &str) {
    run_psql(
        container_name,
        &snapshot_upsert_sql(item_id, user_id, status),
    );
}

fn snapshot_upsert_sql(item_id: &str, user_id: &str, status: &str) -> String {
    format!(
        "INSERT INTO metadata_snapshots (item_id, user_id, title, archive_status)
         VALUES ('{item_id}', '{user_id}', 'Saved title', '{status}')
         ON CONFLICT (item_id)
         DO UPDATE SET archive_status = EXCLUDED.archive_status, updated_at = now();"
    )
}

fn upsert_job(
    container_name: &str,
    item_id: &str,
    user_id: &str,
    status: &str,
    attempt_count: i32,
) {
    run_psql(
        container_name,
        &format!(
            "{} ON CONFLICT (item_id, job_kind)
             DO UPDATE SET
                status = EXCLUDED.status,
                attempt_count = EXCLUDED.attempt_count,
                updated_at = now();",
            job_insert_sql(item_id, user_id, "enrich_metadata", status, attempt_count)
        ),
    );
}

fn job_insert_sql(
    item_id: &str,
    user_id: &str,
    job_kind: &str,
    status: &str,
    attempt_count: i32,
) -> String {
    format!(
        "INSERT INTO processing_jobs (
            item_id, user_id, job_kind, status, attempt_count, idempotency_key
         )
         VALUES (
            '{item_id}', '{user_id}', '{job_kind}', '{status}', {attempt_count},
            '{job_kind}:{item_id}'
         )"
    )
}

fn row_count(container_name: &str, table_name: &str) -> i64 {
    scalar_i64(
        container_name,
        &format!("SELECT count(*) FROM {table_name};"),
    )
}

fn snapshot_status(container_name: &str, item_id: &str) -> String {
    query_value(
        container_name,
        &format!("SELECT archive_status FROM metadata_snapshots WHERE item_id = '{item_id}';"),
    )
}

fn job_attempt_count(container_name: &str, item_id: &str) -> i64 {
    scalar_i64(
        container_name,
        &format!("SELECT attempt_count FROM processing_jobs WHERE item_id = '{item_id}';"),
    )
}

fn query_value(container_name: &str, sql: &str) -> String {
    run_psql(container_name, sql)
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap()
        .trim()
        .to_string()
}

fn scalar_i64(container_name: &str, query: &str) -> i64 {
    query_value(container_name, query).parse().unwrap()
}

fn run_psql(container_name: &str, sql: &str) -> String {
    let output = psql(container_name, sql);
    assert!(
        output.status.success(),
        "psql failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).unwrap()
}

fn assert_psql_fails(container_name: &str, sql: &str) {
    let output = psql(container_name, sql);
    assert!(
        !output.status.success(),
        "psql unexpectedly succeeded\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
