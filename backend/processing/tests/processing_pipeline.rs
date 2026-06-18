use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use processing::extractors::{ExtractedMetadata, MetadataExtractionError, MetadataSource};
use processing::snapshot_store::{SnapshotStoreError, StoredThumbnail, ThumbnailStore};
use processing::{MetadataExtractor, ProcessingPipeline};
use shared::db::LINKDROP_MODEL_MIGRATION;
use shared::processing::ProcessingRepository;
use uuid::Uuid;

static CONTAINER_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[tokio::test]
async fn processing_pipeline_records_snapshot_success_and_failure() {
    let container = setup_sqlx_postgres();
    run_psql(&container.name, LINKDROP_MODEL_MIGRATION);
    let pool = sqlx::PgPool::connect(&database_url(&container))
        .await
        .unwrap();
    let success_item = seed_item(&pool, "https://example.com/success").await;
    let failed_item = seed_item(&pool, "https://example.com/failure").await;
    let pipeline = ProcessingPipeline::new(
        ProcessingRepository::new(pool.clone()),
        FakeExtractor,
        FakeThumbnailStore,
        "worker-test",
    );

    pipeline.process_item(success_item).await.unwrap();
    pipeline.process_item(success_item).await.unwrap();
    pipeline.process_item(failed_item).await.unwrap();

    assert_eq!(snapshot_count(&pool).await, 2);
    assert_eq!(snapshot_status(&pool, success_item).await, "succeeded");
    assert_eq!(snapshot_status(&pool, failed_item).await, "failed");
    assert_eq!(
        snapshot_thumbnail_key(&pool, success_item).await.as_deref(),
        Some("snapshots/success/thumbnail.jpg")
    );
    assert_eq!(
        job_status(&pool, success_item, "enrich_metadata").await,
        "succeeded"
    );
    assert_eq!(
        job_status(&pool, success_item, "snapshot_thumbnail").await,
        "succeeded"
    );
    assert_eq!(
        job_status(&pool, failed_item, "enrich_metadata").await,
        "failed"
    );
}

struct FakeExtractor;

#[async_trait]
impl MetadataExtractor for FakeExtractor {
    async fn extract(
        &self,
        source: MetadataSource,
    ) -> Result<ExtractedMetadata, MetadataExtractionError> {
        if source.url.contains("failure") {
            return Err(MetadataExtractionError::NoMetadataFound);
        }
        Ok(ExtractedMetadata {
            title: Some("Saved title".to_string()),
            thumbnail_url: Some("https://cdn.example.test/thumb.jpg".to_string()),
            author: Some("Creator".to_string()),
            platform: Some("Example".to_string()),
            duration_seconds: Some(90),
        })
    }
}

struct FakeThumbnailStore;

#[async_trait]
impl ThumbnailStore for FakeThumbnailStore {
    async fn store_thumbnail(
        &self,
        _item_id: Uuid,
        _source_url: &str,
    ) -> Result<StoredThumbnail, SnapshotStoreError> {
        Ok(StoredThumbnail {
            key: "snapshots/success/thumbnail.jpg".to_string(),
            content_type: "image/jpeg".to_string(),
        })
    }
}

async fn seed_item(pool: &sqlx::PgPool, canonical_url: &str) -> Uuid {
    let user_id: Uuid =
        sqlx::query_scalar("INSERT INTO users (cognito_sub) VALUES ($1) RETURNING id")
            .bind(format!("user-{canonical_url}"))
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
    .bind(format!("{canonical_url}?utm_source=share"))
    .bind(canonical_url)
    .execute(pool)
    .await
    .unwrap();
    item_id
}

async fn snapshot_count(pool: &sqlx::PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM metadata_snapshots")
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn snapshot_status(pool: &sqlx::PgPool, item_id: Uuid) -> String {
    sqlx::query_scalar("SELECT archive_status FROM metadata_snapshots WHERE item_id = $1")
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

async fn job_status(pool: &sqlx::PgPool, item_id: Uuid, job_kind: &str) -> String {
    sqlx::query_scalar("SELECT status FROM processing_jobs WHERE item_id = $1 AND job_kind = $2")
        .bind(item_id)
        .bind(job_kind)
        .fetch_one(pool)
        .await
        .unwrap()
}

struct SqlxPostgres {
    name: String,
    host: String,
    host_port: u16,
}

impl Drop for SqlxPostgres {
    fn drop(&mut self) {
        let _ = Command::new("docker")
            .args(["rm", "-f", &self.name])
            .status();
    }
}

fn setup_sqlx_postgres() -> SqlxPostgres {
    let sequence = CONTAINER_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let host = docker_host_gateway();
    let host_port = host_port(sequence);
    let name = format!(
        "linkdrop-processing-pipeline-{}-{suffix}-{sequence}",
        std::process::id()
    );
    let port_mapping = format!("0.0.0.0:{host_port}:5432");

    let output = Command::new("docker")
        .args([
            "run",
            "-d",
            "--rm",
            "--name",
            &name,
            "-p",
            &port_mapping,
            "-e",
            "POSTGRES_PASSWORD=postgres",
            "postgres:16-alpine",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "docker run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    wait_for_host_port(&host, host_port);
    wait_for_psql(&name);
    SqlxPostgres {
        name,
        host,
        host_port,
    }
}

fn database_url(container: &SqlxPostgres) -> String {
    format!(
        "postgres://postgres:postgres@{}:{}/postgres?sslmode=disable",
        container.host, container.host_port
    )
}

fn run_psql(container_name: &str, sql: &str) {
    let output = Command::new("docker")
        .args([
            "run",
            "--rm",
            "--network",
            &format!("container:{container_name}"),
            "-e",
            "PGPASSWORD=postgres",
            "postgres:16-alpine",
            "psql",
            "-h",
            "127.0.0.1",
            "-U",
            "postgres",
            "-d",
            "postgres",
            "-v",
            "ON_ERROR_STOP=1",
            "-qAt",
            "-c",
            sql,
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "psql failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn host_port(sequence: u64) -> u16 {
    let base = 30_000 + (u64::from(std::process::id()) % 10_000);
    (base + sequence) as u16
}

fn docker_host_gateway() -> String {
    fs::read_to_string("/proc/net/route")
        .unwrap()
        .lines()
        .skip(1)
        .find_map(default_gateway)
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

fn default_gateway(line: &str) -> Option<String> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.get(1) != Some(&"00000000") {
        return None;
    }
    let raw = u32::from_str_radix(fields.get(2)?, 16).ok()?;
    Some(IpAddr::V4(Ipv4Addr::from(raw.to_le_bytes())).to_string())
}

fn wait_for_host_port(host: &str, host_port: u16) {
    let address = SocketAddr::new(host.parse().unwrap(), host_port);
    for _ in 0..60 {
        if TcpStream::connect_timeout(&address, Duration::from_millis(500)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    panic!("Postgres host port did not become ready");
}

fn wait_for_psql(container_name: &str) {
    for _ in 0..60 {
        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "--network",
                &format!("container:{container_name}"),
                "-e",
                "PGPASSWORD=postgres",
                "postgres:16-alpine",
                "pg_isready",
                "-h",
                "127.0.0.1",
                "-U",
                "postgres",
            ])
            .output()
            .unwrap();
        if output.status.success() {
            return;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    panic!("Postgres did not become ready for psql");
}
