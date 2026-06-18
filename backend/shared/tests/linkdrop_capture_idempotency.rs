mod support;

use shared::db::{LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION, LINKDROP_MODEL_MIGRATION};
use support::{psql, setup_postgres};

#[test]
fn linkdrop_capture_idempotency_prevents_duplicate_share_retries() {
    let container = setup_postgres();
    run_psql(&container.name, LINKDROP_MODEL_MIGRATION);
    run_psql(&container.name, LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION);

    let user_id = insert_user(&container.name, "capture-user");
    let item_id = insert_item_with_capture_id(&container.name, &user_id, "share-attempt-1");
    insert_url(
        &container.name,
        &item_id,
        &user_id,
        "https://example.com/same",
    );

    assert_psql_fails(
        &container.name,
        &format!(
            "INSERT INTO items (user_id, client_capture_id)
             VALUES ('{user_id}', 'share-attempt-1');"
        ),
    );
    assert_psql_fails(
        &container.name,
        &format!(
            "INSERT INTO items (user_id, client_capture_id)
             VALUES ('{user_id}', '   ');"
        ),
    );

    let first_pending = insert_item_without_capture_id(&container.name, &user_id);
    let second_pending = insert_item_without_capture_id(&container.name, &user_id);
    insert_url(
        &container.name,
        &first_pending,
        &user_id,
        "https://example.com/repeated-original",
    );
    insert_url(
        &container.name,
        &second_pending,
        &user_id,
        "https://example.com/repeated-original",
    );

    assert_eq!(
        scalar_i64(
            &container.name,
            "SELECT count(*)
             FROM item_urls
             WHERE original_url = 'https://example.com/repeated-original'
               AND canonical_url IS NULL;"
        ),
        2
    );
}

fn insert_user(container_name: &str, cognito_sub: &str) -> String {
    query_value(
        container_name,
        &format!("INSERT INTO users (cognito_sub) VALUES ('{cognito_sub}') RETURNING id;"),
    )
}

fn insert_item_with_capture_id(container_name: &str, user_id: &str, capture_id: &str) -> String {
    query_value(
        container_name,
        &format!(
            "INSERT INTO items (user_id, client_capture_id)
             VALUES ('{user_id}', '{capture_id}') RETURNING id;"
        ),
    )
}

fn insert_item_without_capture_id(container_name: &str, user_id: &str) -> String {
    query_value(
        container_name,
        &format!("INSERT INTO items (user_id) VALUES ('{user_id}') RETURNING id;"),
    )
}

fn insert_url(container_name: &str, item_id: &str, user_id: &str, original_url: &str) {
    run_psql(
        container_name,
        &format!(
            "INSERT INTO item_urls (item_id, user_id, original_url)
             VALUES ('{item_id}', '{user_id}', '{original_url}');"
        ),
    );
}

fn scalar_i64(container_name: &str, query: &str) -> i64 {
    query_value(container_name, query).parse().unwrap()
}

fn query_value(container_name: &str, sql: &str) -> String {
    run_psql(container_name, sql)
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap()
        .trim()
        .to_string()
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
