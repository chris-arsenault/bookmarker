mod support;

use shared::db::LINKDROP_MODEL_MIGRATION;
use support::{psql, setup_postgres};

#[test]
fn linkdrop_model_enforces_owned_items_and_canonical_dedup_keys() {
    let container = setup_postgres();
    run_psql(&container.name, LINKDROP_MODEL_MIGRATION);

    let user_a = insert_user(&container.name, "user-a");
    let user_b = insert_user(&container.name, "user-b");
    assert_psql_fails(
        &container.name,
        "INSERT INTO users (cognito_sub) VALUES ('user-a');",
    );

    let item_a = insert_item(&container.name, &user_a);
    let item_a_duplicate = insert_item(&container.name, &user_a);
    let item_b = insert_item(&container.name, &user_b);
    assert_psql_fails(
        &container.name,
        &format!(
            "INSERT INTO item_urls (item_id, user_id, original_url)
             VALUES ('{item_a}', '{user_b}', 'https://example.com/wrong-owner');"
        ),
    );

    insert_url(
        &container.name,
        &item_a,
        &user_a,
        "https://youtu.be/abc",
        "https://youtube.com/watch?v=abc",
    );
    assert_psql_fails(
        &container.name,
        &format!(
            "INSERT INTO item_urls (item_id, user_id, original_url, canonical_url)
             VALUES (
                '{item_a_duplicate}', '{user_a}',
                'https://m.youtube.com/watch?v=abc',
                'https://youtube.com/watch?v=abc'
             );"
        ),
    );

    insert_url(
        &container.name,
        &item_b,
        &user_b,
        "https://m.youtube.com/watch?v=abc",
        "https://youtube.com/watch?v=abc",
    );
    insert_pending_url(
        &container.name,
        &insert_item(&container.name, &user_a),
        &user_a,
    );
    insert_pending_url(
        &container.name,
        &insert_item(&container.name, &user_a),
        &user_a,
    );
}

fn insert_user(container_name: &str, cognito_sub: &str) -> String {
    query_value(
        container_name,
        &format!("INSERT INTO users (cognito_sub) VALUES ('{cognito_sub}') RETURNING id;"),
    )
}

fn insert_item(container_name: &str, user_id: &str) -> String {
    query_value(
        container_name,
        &format!("INSERT INTO items (user_id) VALUES ('{user_id}') RETURNING id;"),
    )
}

fn insert_url(
    container_name: &str,
    item_id: &str,
    user_id: &str,
    original_url: &str,
    canonical_url: &str,
) {
    run_psql(
        container_name,
        &format!(
            "INSERT INTO item_urls (item_id, user_id, original_url, canonical_url)
             VALUES ('{item_id}', '{user_id}', '{original_url}', '{canonical_url}');"
        ),
    );
}

fn insert_pending_url(container_name: &str, item_id: &str, user_id: &str) {
    run_psql(
        container_name,
        &format!(
            "INSERT INTO item_urls (item_id, user_id, original_url)
             VALUES ('{item_id}', '{user_id}', 'https://example.com/pending');"
        ),
    );
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
