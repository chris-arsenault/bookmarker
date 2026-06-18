mod support;

use shared::db::{LINKDROP_MODEL_MIGRATION, LINKDROP_MODEL_ROLLBACK};
use support::{psql, setup_postgres};

const LINKDROP_TABLES: &[&str] = &[
    "users",
    "items",
    "item_urls",
    "tags",
    "item_tags",
    "tag_usage_counts",
    "item_notes",
    "metadata_snapshots",
    "processing_jobs",
];

#[test]
fn linkdrop_migration_round_trip_applies_and_rolls_back() {
    let container = setup_postgres();

    run_psql(&container.name, LINKDROP_MODEL_MIGRATION);
    for table_name in LINKDROP_TABLES {
        assert!(
            table_exists(&container.name, table_name),
            "{table_name} exists"
        );
    }

    run_psql(&container.name, LINKDROP_MODEL_ROLLBACK);
    for table_name in LINKDROP_TABLES.iter().rev() {
        assert!(
            !table_exists(&container.name, table_name),
            "{table_name} removed"
        );
    }

    run_psql(&container.name, LINKDROP_MODEL_MIGRATION);
    assert!(table_exists(&container.name, "items"));
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

fn scalar_i64(container_name: &str, query: &str) -> i64 {
    run_psql(container_name, query)
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap()
        .trim()
        .parse()
        .unwrap()
}

fn table_exists(container_name: &str, table_name: &str) -> bool {
    let query = format!(
        "SELECT CASE WHEN EXISTS (
            SELECT 1
            FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = '{table_name}'
        ) THEN 1 ELSE 0 END;"
    );
    scalar_i64(container_name, &query) == 1
}
