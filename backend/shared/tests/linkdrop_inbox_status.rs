mod support;

use shared::db::{LINKDROP_INBOX_STATUS_MIGRATION, LINKDROP_MODEL_MIGRATION};
use support::{psql, setup_postgres};

#[test]
fn linkdrop_model_adds_unsorted_inbox_status() {
    let container = setup_postgres();
    run_psql(&container.name, LINKDROP_MODEL_MIGRATION);
    run_psql(&container.name, LINKDROP_INBOX_STATUS_MIGRATION);

    let user_id = query_value(
        &container.name,
        "INSERT INTO users (cognito_sub) VALUES ('inbox-user') RETURNING id;",
    );
    let item_id = query_value(
        &container.name,
        &format!("INSERT INTO items (user_id) VALUES ('{user_id}') RETURNING id;"),
    );

    assert_eq!(
        query_value(
            &container.name,
            &format!("SELECT inbox_status FROM items WHERE id = '{item_id}';"),
        ),
        "unsorted"
    );
    assert_psql_fails(
        &container.name,
        &format!("UPDATE items SET inbox_status = 'hidden' WHERE id = '{item_id}';"),
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
