#[path = "support/sqlx.rs"]
mod sqlx_support;

use shared::auth::UserContext;
use shared::db::{
    LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION, LINKDROP_INBOX_STATUS_MIGRATION,
    LINKDROP_MODEL_MIGRATION, LINKDROP_TEXT_SNIPPET_MIGRATION,
};
use shared::domain::ArchiveStatus;
use shared::library::{CaptureItemRequest, LibraryService, ListItemsQuery};
use shared::library_pg::PgLibraryService;
use sqlx_support::{database_url, psql, setup_sqlx_postgres};

#[tokio::test]
async fn pg_capture_persists_optional_link_title() {
    let container = setup_sqlx_postgres();
    apply_migrations(&container.name);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let service = PgLibraryService::new(pool);

    let outcome = service
        .capture_item(
            &user(),
            CaptureItemRequest {
                url: "https://example.com/titled".to_string(),
                title: Some(" Shared page title ".to_string()),
                tags: Vec::new(),
                client_capture_id: Some("share-title-1".to_string()),
            },
        )
        .await
        .unwrap();

    assert!(outcome.created);
    assert_eq!(
        outcome.item.summary.title.as_deref(),
        Some("Shared page title")
    );
    assert_eq!(outcome.item.summary.archive_status, ArchiveStatus::Pending);
    let listed = service
        .list_items(&user(), &ListItemsQuery::default())
        .await
        .unwrap();
    assert_eq!(listed[0].title.as_deref(), Some("Shared page title"));
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
