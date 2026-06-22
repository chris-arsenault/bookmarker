#[path = "support/sqlx.rs"]
mod sqlx_support;

use shared::auth::UserContext;
use shared::db::{
    LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION, LINKDROP_IMAGE_ITEMS_MIGRATION,
    LINKDROP_INBOX_STATUS_MIGRATION, LINKDROP_ITEM_TITLES_MIGRATION, LINKDROP_MODEL_MIGRATION,
    LINKDROP_TEXT_SNIPPET_MIGRATION,
};
use shared::domain::{ArchiveStatus, ImageUploadStatus, ItemKind};
use shared::library::{CaptureImageUploadRequest, LibraryService, ListItemsQuery};
use shared::library_pg::PgLibraryService;
use sqlx_support::{database_url, psql, setup_sqlx_postgres};

#[tokio::test]
async fn pg_capture_image_upload_persists_metadata_and_completion() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let service = PgLibraryService::new(pool);

    let first = service
        .capture_image_upload(&user(), image_request("image-share-1"))
        .await
        .unwrap();
    let repeated = service
        .capture_image_upload(&user(), image_request("image-share-1"))
        .await
        .unwrap();

    assert!(first.created);
    assert!(!repeated.created);
    assert_eq!(first.item.summary.id, repeated.item.summary.id);
    assert_eq!(first.item.summary.item_kind, ItemKind::Image);
    assert_eq!(first.item.summary.archive_status, ArchiveStatus::Pending);
    let image = first.item.summary.image.as_ref().unwrap();
    assert_eq!(image.content_type, "image/jpeg");
    assert_eq!(image.original_filename.as_deref(), Some("phone.jpg"));
    assert_eq!(image.byte_size, Some(2048));
    assert_eq!(image.upload_status, ImageUploadStatus::Pending);
    assert!(image.s3_key.starts_with("images/"));

    let completed = service
        .complete_image_upload(&user(), first.item.summary.id)
        .await
        .unwrap();
    assert_eq!(completed.summary.archive_status, ArchiveStatus::Succeeded);
    assert_eq!(
        completed.summary.image.unwrap().upload_status,
        ImageUploadStatus::Uploaded
    );
    assert_eq!(matching_image_count(&service).await, 1);
}

fn image_request(client_capture_id: &str) -> CaptureImageUploadRequest {
    CaptureImageUploadRequest {
        content_type: "image/jpeg".to_string(),
        title: Some("Phone transfer".to_string()),
        original_filename: Some("phone.jpg".to_string()),
        byte_size: Some(2048),
        source_app: Some("Android share".to_string()),
        source_device: Some("android".to_string()),
        capture_method: Some("android_share".to_string()),
        tags: vec!["Photos".to_string()],
        client_capture_id: Some(client_capture_id.to_string()),
    }
}

async fn matching_image_count(service: &PgLibraryService) -> usize {
    service
        .list_items(
            &user(),
            &ListItemsQuery {
                q: Some("phone".to_string()),
                ..ListItemsQuery::default()
            },
        )
        .await
        .unwrap()
        .len()
}

fn user() -> UserContext {
    UserContext {
        sub: "image-user".to_string(),
        email: Some("image@example.test".to_string()),
        username: Some("image-user".to_string()),
        groups: vec![],
    }
}

fn apply_migrations(container_name: &str) {
    run_psql(container_name, LINKDROP_MODEL_MIGRATION);
    run_psql(container_name, LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION);
    run_psql(container_name, LINKDROP_INBOX_STATUS_MIGRATION);
    run_psql(container_name, LINKDROP_TEXT_SNIPPET_MIGRATION);
    run_psql(container_name, LINKDROP_ITEM_TITLES_MIGRATION);
    run_psql(container_name, LINKDROP_IMAGE_ITEMS_MIGRATION);
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
