use uuid::Uuid;

use crate::db::DbPool;
use crate::domain::{ArchiveStatus, ProcessingJobKind, ProcessingStatus};
use crate::error::{AppError, AppResult};

const ENQUEUE_JOB: &str = "
    INSERT INTO processing_jobs (
        item_id,
        user_id,
        job_kind,
        status,
        attempt_count,
        idempotency_key,
        available_at,
        locked_at,
        locked_by,
        last_error
    )
    SELECT id, user_id, $2, 'queued', 0, $3, now(), NULL, NULL, NULL
    FROM items
    WHERE id = $1
    ON CONFLICT (item_id, job_kind)
    DO UPDATE SET
        status = 'queued',
        available_at = now(),
        locked_at = NULL,
        locked_by = NULL,
        last_error = NULL,
        updated_at = now()
    RETURNING item_id, job_kind, status, attempt_count, last_error";
const LOAD_ITEM: &str = "
    SELECT
        items.id AS item_id,
        items.user_id,
        item_urls.original_url,
        item_urls.canonical_url
    FROM items
    JOIN item_urls ON item_urls.item_id = items.id
    WHERE items.id = $1";
const MARK_JOB_RUNNING: &str = "
    UPDATE processing_jobs
    SET
        status = 'running',
        attempt_count = attempt_count + 1,
        locked_at = now(),
        locked_by = $3,
        updated_at = now()
    WHERE item_id = $1 AND job_kind = $2
    RETURNING item_id, job_kind, status, attempt_count, last_error";
const MARK_JOB_TERMINAL: &str = "
    UPDATE processing_jobs
    SET
        status = $3,
        last_error = $4,
        locked_at = NULL,
        locked_by = NULL,
        updated_at = now()
    WHERE item_id = $1 AND job_kind = $2
    RETURNING item_id, job_kind, status, attempt_count, last_error";
const LOAD_JOB_STATE: &str = "
    SELECT item_id, job_kind, status, attempt_count, last_error
    FROM processing_jobs
    WHERE item_id = $1 AND job_kind = $2";
const UPSERT_SNAPSHOT: &str = "
    INSERT INTO metadata_snapshots (
        item_id,
        user_id,
        title,
        thumbnail_s3_key,
        thumbnail_content_type,
        author,
        platform,
        duration_seconds,
        archive_status,
        archive_error,
        captured_at
    )
    SELECT
        id,
        user_id,
        $2,
        $3,
        $4,
        $5,
        $6,
        $7,
        $8,
        $9,
        CASE WHEN $8 = 'succeeded' THEN now() ELSE NULL END
    FROM items
    WHERE id = $1
    ON CONFLICT (item_id)
    DO UPDATE SET
        title = COALESCE(EXCLUDED.title, metadata_snapshots.title),
        thumbnail_s3_key = COALESCE(
            EXCLUDED.thumbnail_s3_key,
            metadata_snapshots.thumbnail_s3_key
        ),
        thumbnail_content_type = COALESCE(
            EXCLUDED.thumbnail_content_type,
            metadata_snapshots.thumbnail_content_type
        ),
        author = COALESCE(EXCLUDED.author, metadata_snapshots.author),
        platform = COALESCE(EXCLUDED.platform, metadata_snapshots.platform),
        duration_seconds = COALESCE(
            EXCLUDED.duration_seconds,
            metadata_snapshots.duration_seconds
        ),
        archive_status = EXCLUDED.archive_status,
        archive_error = EXCLUDED.archive_error,
        captured_at = COALESCE(EXCLUDED.captured_at, metadata_snapshots.captured_at),
        updated_at = now()
    RETURNING item_id";

#[derive(Clone)]
pub struct ProcessingRepository {
    db: DbPool,
}

impl ProcessingRepository {
    pub fn new(db: DbPool) -> Self {
        Self { db }
    }

    pub async fn enqueue_job(
        &self,
        item_id: Uuid,
        job_kind: ProcessingJobKind,
    ) -> AppResult<ProcessingJobState> {
        let row = sqlx::query_as(ENQUEUE_JOB)
            .bind(item_id)
            .bind(job_kind.as_str())
            .bind(idempotency_key(item_id, job_kind))
            .fetch_optional(&self.db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(item_id))?;
        job_state(row)
    }

    pub async fn load_item(&self, item_id: Uuid) -> AppResult<ProcessingItem> {
        let row: ProcessingItemRow = sqlx::query_as(LOAD_ITEM)
            .bind(item_id)
            .fetch_optional(&self.db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(item_id))?;
        Ok(row.into_item())
    }

    pub async fn mark_job_running(
        &self,
        item_id: Uuid,
        job_kind: ProcessingJobKind,
        worker_id: &str,
    ) -> AppResult<ProcessingJobState> {
        let row = sqlx::query_as(MARK_JOB_RUNNING)
            .bind(item_id)
            .bind(job_kind.as_str())
            .bind(worker_id)
            .fetch_optional(&self.db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(item_id))?;
        job_state(row)
    }

    pub async fn mark_job_succeeded(
        &self,
        item_id: Uuid,
        job_kind: ProcessingJobKind,
    ) -> AppResult<ProcessingJobState> {
        self.mark_job_terminal(item_id, job_kind, ProcessingStatus::Succeeded, None)
            .await
    }

    pub async fn mark_job_failed(
        &self,
        item_id: Uuid,
        job_kind: ProcessingJobKind,
        error: &str,
    ) -> AppResult<ProcessingJobState> {
        self.mark_job_terminal(item_id, job_kind, ProcessingStatus::Failed, Some(error))
            .await
    }

    pub async fn load_job_state(
        &self,
        item_id: Uuid,
        job_kind: ProcessingJobKind,
    ) -> AppResult<Option<ProcessingJobState>> {
        let row = sqlx::query_as(LOAD_JOB_STATE)
            .bind(item_id)
            .bind(job_kind.as_str())
            .fetch_optional(&self.db)
            .await
            .map_err(database_error)?;
        row.map(job_state).transpose()
    }

    pub async fn upsert_snapshot(&self, item_id: Uuid, update: SnapshotUpdate) -> AppResult<()> {
        sqlx::query_scalar::<_, Uuid>(UPSERT_SNAPSHOT)
            .bind(item_id)
            .bind(update.title.as_deref())
            .bind(update.thumbnail_s3_key.as_deref())
            .bind(update.thumbnail_content_type.as_deref())
            .bind(update.author.as_deref())
            .bind(update.platform.as_deref())
            .bind(update.duration_seconds)
            .bind(update.archive_status.as_str())
            .bind(update.archive_error.as_deref())
            .fetch_optional(&self.db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(item_id))?;
        Ok(())
    }

    async fn mark_job_terminal(
        &self,
        item_id: Uuid,
        job_kind: ProcessingJobKind,
        status: ProcessingStatus,
        error: Option<&str>,
    ) -> AppResult<ProcessingJobState> {
        let row = sqlx::query_as(MARK_JOB_TERMINAL)
            .bind(item_id)
            .bind(job_kind.as_str())
            .bind(status.as_str())
            .bind(error)
            .fetch_optional(&self.db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(item_id))?;
        job_state(row)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingItem {
    pub item_id: Uuid,
    pub user_id: Uuid,
    pub original_url: String,
    pub canonical_url: Option<String>,
    pub source_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessingJobState {
    pub item_id: Uuid,
    pub job_kind: ProcessingJobKind,
    pub status: ProcessingStatus,
    pub attempt_count: i32,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotUpdate {
    pub title: Option<String>,
    pub thumbnail_s3_key: Option<String>,
    pub thumbnail_content_type: Option<String>,
    pub author: Option<String>,
    pub platform: Option<String>,
    pub duration_seconds: Option<i32>,
    pub archive_status: ArchiveStatus,
    pub archive_error: Option<String>,
}

impl Default for SnapshotUpdate {
    fn default() -> Self {
        Self {
            title: None,
            thumbnail_s3_key: None,
            thumbnail_content_type: None,
            author: None,
            platform: None,
            duration_seconds: None,
            archive_status: ArchiveStatus::Pending,
            archive_error: None,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ProcessingItemRow {
    item_id: Uuid,
    user_id: Uuid,
    original_url: String,
    canonical_url: Option<String>,
}

impl ProcessingItemRow {
    fn into_item(self) -> ProcessingItem {
        let source_url = self
            .canonical_url
            .clone()
            .unwrap_or_else(|| self.original_url.clone());
        ProcessingItem {
            item_id: self.item_id,
            user_id: self.user_id,
            original_url: self.original_url,
            canonical_url: self.canonical_url,
            source_url,
        }
    }
}

#[derive(sqlx::FromRow)]
struct ProcessingJobRow {
    item_id: Uuid,
    job_kind: String,
    status: String,
    attempt_count: i32,
    last_error: Option<String>,
}

fn job_state(row: ProcessingJobRow) -> AppResult<ProcessingJobState> {
    Ok(ProcessingJobState {
        item_id: row.item_id,
        job_kind: ProcessingJobKind::try_from(row.job_kind.as_str())
            .map_err(|err| AppError::Database(err.to_string()))?,
        status: ProcessingStatus::try_from(row.status.as_str())
            .map_err(|err| AppError::Database(err.to_string()))?,
        attempt_count: row.attempt_count,
        last_error: row.last_error,
    })
}

fn idempotency_key(item_id: Uuid, job_kind: ProcessingJobKind) -> String {
    format!("{}:{item_id}", job_kind.as_str())
}

fn database_error(err: sqlx::Error) -> AppError {
    AppError::Database(err.to_string())
}

fn not_found(item_id: Uuid) -> AppError {
    AppError::NotFound(format!("item {item_id}"))
}
