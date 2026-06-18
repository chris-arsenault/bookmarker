mod support;

use shared::db::LINKDROP_MODEL_MIGRATION;
use support::{psql, setup_postgres};

#[test]
fn linkdrop_model_maintains_explicit_tag_usage_and_merge_invariants() {
    let container = setup_postgres();
    run_psql(&container.name, LINKDROP_MODEL_MIGRATION);

    let user_id = insert_user(&container.name, "tag-user");
    assert_eq!(
        scalar_i64(&container.name, "SELECT count(*) FROM tag_usage_counts;"),
        0
    );

    let target_tag = insert_tag(&container.name, &user_id, "Learning");
    assert_psql_fails(
        &container.name,
        &format!("INSERT INTO tags (user_id, display_name) VALUES ('{user_id}', ' learning ');"),
    );

    let item_one = insert_item(&container.name, &user_id);
    let item_two = insert_item(&container.name, &user_id);
    apply_tag(&container.name, &item_one, &target_tag, &user_id);
    apply_tag(&container.name, &item_two, &target_tag, &user_id);
    apply_tag_noop(&container.name, &item_two, &target_tag, &user_id);
    assert_eq!(usage_count(&container.name, &target_tag), 2);

    delete_tag_edge(&container.name, &item_two, &target_tag);
    assert_eq!(usage_count(&container.name, &target_tag), 1);
    apply_tag(&container.name, &item_two, &target_tag, &user_id);

    let source_tag = insert_tag(&container.name, &user_id, "Videos");
    let item_three = insert_item(&container.name, &user_id);
    apply_tag(&container.name, &item_two, &source_tag, &user_id);
    apply_tag(&container.name, &item_three, &source_tag, &user_id);
    merge_tags(&container.name, &source_tag, &target_tag);

    assert_eq!(usage_count(&container.name, &target_tag), 3);
    assert_eq!(
        scalar_i64(
            &container.name,
            "SELECT count(*) FROM tags WHERE display_name = 'Videos';"
        ),
        0
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

fn insert_tag(container_name: &str, user_id: &str, display_name: &str) -> String {
    query_value(
        container_name,
        &format!(
            "INSERT INTO tags (user_id, display_name)
             VALUES ('{user_id}', '{display_name}') RETURNING id;"
        ),
    )
}

fn apply_tag(container_name: &str, item_id: &str, tag_id: &str, user_id: &str) {
    run_psql(
        container_name,
        &format!(
            "INSERT INTO item_tags (item_id, tag_id, user_id)
             VALUES ('{item_id}', '{tag_id}', '{user_id}');"
        ),
    );
}

fn apply_tag_noop(container_name: &str, item_id: &str, tag_id: &str, user_id: &str) {
    run_psql(
        container_name,
        &format!(
            "INSERT INTO item_tags (item_id, tag_id, user_id)
             VALUES ('{item_id}', '{tag_id}', '{user_id}')
             ON CONFLICT DO NOTHING;"
        ),
    );
}

fn delete_tag_edge(container_name: &str, item_id: &str, tag_id: &str) {
    run_psql(
        container_name,
        &format!("DELETE FROM item_tags WHERE item_id = '{item_id}' AND tag_id = '{tag_id}';"),
    );
}

fn merge_tags(container_name: &str, source_tag: &str, target_tag: &str) {
    run_psql(
        container_name,
        &format!(
            "INSERT INTO item_tags (item_id, tag_id, user_id)
             SELECT item_id, '{target_tag}', user_id
             FROM item_tags
             WHERE tag_id = '{source_tag}'
             ON CONFLICT DO NOTHING;
             DELETE FROM item_tags WHERE tag_id = '{source_tag}';
             DELETE FROM tags WHERE id = '{source_tag}';"
        ),
    );
}

fn usage_count(container_name: &str, tag_id: &str) -> i64 {
    scalar_i64(
        container_name,
        &format!("SELECT usage_count FROM tag_usage_counts WHERE tag_id = '{tag_id}';"),
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
