use sqlx::postgres::PgPoolOptions;

use crate::config::{AppConfig, DatabaseConfig};
use crate::error::{AppError, AppResult};

pub const LINKDROP_MODEL_MIGRATION: &str =
    include_str!("../../../db/migrations/001_create_linkdrop_model.sql");
pub const LINKDROP_MODEL_ROLLBACK: &str =
    include_str!("../../../db/migrations/rollback/001_create_linkdrop_model.sql");
pub const LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION: &str =
    include_str!("../../../db/migrations/002_capture_idempotency.sql");
pub const LINKDROP_CAPTURE_IDEMPOTENCY_ROLLBACK: &str =
    include_str!("../../../db/migrations/rollback/002_capture_idempotency.sql");
pub const LINKDROP_INBOX_STATUS_MIGRATION: &str =
    include_str!("../../../db/migrations/003_item_inbox_status.sql");
pub const LINKDROP_INBOX_STATUS_ROLLBACK: &str =
    include_str!("../../../db/migrations/rollback/003_item_inbox_status.sql");
pub const LINKDROP_TEXT_SNIPPET_MIGRATION: &str =
    include_str!("../../../db/migrations/004_text_snippet_items.sql");
pub const LINKDROP_TEXT_SNIPPET_ROLLBACK: &str =
    include_str!("../../../db/migrations/rollback/004_text_snippet_items.sql");

pub type DbPool = sqlx::PgPool;

const MAX_POOL_CONNECTIONS: u32 = 5;

pub async fn connect_pool(config: &AppConfig) -> AppResult<DbPool> {
    PgPoolOptions::new()
        .max_connections(MAX_POOL_CONNECTIONS)
        .connect(&database_url(&config.database))
        .await
        .map_err(|err| AppError::Database(err.to_string()))
}

pub fn database_url(config: &DatabaseConfig) -> String {
    format!(
        "postgres://{}:{}@{}:{}/{}?sslmode=require",
        encode_userinfo(&config.username),
        encode_userinfo(&config.password),
        config.host,
        config.port,
        config.name
    )
}

fn encode_userinfo(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                vec![byte as char]
            }
            _ => {
                let encoded = format!("%{byte:02X}");
                encoded.chars().collect()
            }
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests {
    use crate::config::DatabaseConfig;

    use super::database_url;
    use super::{
        LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION, LINKDROP_CAPTURE_IDEMPOTENCY_ROLLBACK,
        LINKDROP_INBOX_STATUS_MIGRATION, LINKDROP_INBOX_STATUS_ROLLBACK, LINKDROP_MODEL_MIGRATION,
        LINKDROP_MODEL_ROLLBACK, LINKDROP_TEXT_SNIPPET_MIGRATION, LINKDROP_TEXT_SNIPPET_ROLLBACK,
    };

    #[test]
    fn migration_constants_reference_linkdrop_tables() {
        for table in [
            "users",
            "items",
            "item_urls",
            "tags",
            "item_tags",
            "tag_usage_counts",
            "item_notes",
            "metadata_snapshots",
            "processing_jobs",
        ] {
            assert!(LINKDROP_MODEL_MIGRATION.contains(&format!("CREATE TABLE {table}")));
            assert!(LINKDROP_MODEL_ROLLBACK.contains(&format!("DROP TABLE IF EXISTS {table}")));
        }

        assert!(LINKDROP_CAPTURE_IDEMPOTENCY_MIGRATION.contains("client_capture_id"));
        assert!(LINKDROP_CAPTURE_IDEMPOTENCY_ROLLBACK.contains("client_capture_id"));
        assert!(LINKDROP_INBOX_STATUS_MIGRATION.contains("inbox_status"));
        assert!(LINKDROP_INBOX_STATUS_ROLLBACK.contains("inbox_status"));
        assert!(LINKDROP_TEXT_SNIPPET_MIGRATION.contains("CREATE TABLE item_texts"));
        assert!(LINKDROP_TEXT_SNIPPET_ROLLBACK.contains("DROP TABLE IF EXISTS item_texts"));
    }

    #[test]
    fn database_url_uses_platform_env_config_with_tls_required() {
        let config = DatabaseConfig {
            host: "db.internal".to_string(),
            port: 6543,
            name: "linkdrop".to_string(),
            username: "app_user".to_string(),
            password: "p@ss word".to_string(),
        };

        assert_eq!(
            database_url(&config),
            "postgres://app_user:p%40ss%20word@db.internal:6543/linkdrop?sslmode=require"
        );
    }
}
