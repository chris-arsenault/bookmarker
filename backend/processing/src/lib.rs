pub mod extractors;
pub mod snapshot_store;

use async_trait::async_trait;
use extractors::{
    ExtractedMetadata, MetadataExtractionError, MetadataFetch, MetadataSource, OpenGraphExtractor,
};
use shared::domain::{ArchiveStatus, ProcessingJobKind};
use shared::error::AppResult;
use shared::processing::{ProcessingItem, ProcessingRepository, SnapshotUpdate};
use snapshot_store::{StoredThumbnail, ThumbnailStore};
use uuid::Uuid;

#[async_trait]
pub trait MetadataExtractor: Send + Sync {
    async fn extract(
        &self,
        source: MetadataSource,
    ) -> Result<ExtractedMetadata, MetadataExtractionError>;
}

#[async_trait]
impl<F> MetadataExtractor for OpenGraphExtractor<F>
where
    F: MetadataFetch,
{
    async fn extract(
        &self,
        source: MetadataSource,
    ) -> Result<ExtractedMetadata, MetadataExtractionError> {
        OpenGraphExtractor::extract(self, source).await
    }
}

pub struct ProcessingPipeline<E, S> {
    repository: ProcessingRepository,
    extractor: E,
    thumbnail_store: S,
    worker_id: String,
}

impl<E, S> ProcessingPipeline<E, S>
where
    E: MetadataExtractor,
    S: ThumbnailStore,
{
    pub fn new(
        repository: ProcessingRepository,
        extractor: E,
        thumbnail_store: S,
        worker_id: impl Into<String>,
    ) -> Self {
        Self {
            repository,
            extractor,
            thumbnail_store,
            worker_id: worker_id.into(),
        }
    }

    pub async fn process_item(&self, item_id: Uuid) -> AppResult<()> {
        self.start_job(item_id, ProcessingJobKind::EnrichMetadata)
            .await?;
        let item = self.repository.load_item(item_id).await?;
        match self.extract_metadata(&item).await {
            Ok(metadata) => self.record_metadata(item_id, metadata).await,
            Err(error) => self.fail_enrichment(item_id, error).await,
        }
    }

    async fn extract_metadata(
        &self,
        item: &ProcessingItem,
    ) -> Result<ExtractedMetadata, MetadataExtractionError> {
        self.extractor
            .extract(MetadataSource::new(item.source_url.clone()))
            .await
    }

    async fn record_metadata(&self, item_id: Uuid, metadata: ExtractedMetadata) -> AppResult<()> {
        match self
            .store_thumbnail(item_id, metadata.thumbnail_url.as_deref())
            .await
        {
            Ok(thumbnail) => self.succeed_enrichment(item_id, metadata, thumbnail).await,
            Err(error) => self.fail_snapshot(item_id, metadata, error).await,
        }
    }

    async fn store_thumbnail(
        &self,
        item_id: Uuid,
        thumbnail_url: Option<&str>,
    ) -> Result<Option<StoredThumbnail>, String> {
        let Some(url) = thumbnail_url else {
            return Ok(None);
        };
        self.start_job(item_id, ProcessingJobKind::SnapshotThumbnail)
            .await
            .map_err(|error| error.to_string())?;
        self.thumbnail_store
            .store_thumbnail(item_id, url)
            .await
            .map(Some)
            .map_err(safe_error)
    }

    async fn succeed_enrichment(
        &self,
        item_id: Uuid,
        metadata: ExtractedMetadata,
        thumbnail: Option<StoredThumbnail>,
    ) -> AppResult<()> {
        self.repository
            .upsert_snapshot(item_id, successful_snapshot(metadata, thumbnail))
            .await?;
        self.finish_snapshot_job(item_id, ProcessingOutcome::Succeeded)
            .await?;
        self.repository
            .mark_job_succeeded(item_id, ProcessingJobKind::EnrichMetadata)
            .await?;
        Ok(())
    }

    async fn fail_snapshot(
        &self,
        item_id: Uuid,
        metadata: ExtractedMetadata,
        error: String,
    ) -> AppResult<()> {
        self.repository
            .upsert_snapshot(item_id, failed_snapshot(metadata, &error))
            .await?;
        self.finish_snapshot_job(item_id, ProcessingOutcome::Failed(&error))
            .await?;
        self.repository
            .mark_job_failed(item_id, ProcessingJobKind::EnrichMetadata, &error)
            .await?;
        Ok(())
    }

    async fn fail_enrichment(
        &self,
        item_id: Uuid,
        error: MetadataExtractionError,
    ) -> AppResult<()> {
        let message = safe_error(error);
        self.repository
            .upsert_snapshot(item_id, failed_enrichment_snapshot(&message))
            .await?;
        self.repository
            .mark_job_failed(item_id, ProcessingJobKind::EnrichMetadata, &message)
            .await?;
        Ok(())
    }

    async fn start_job(&self, item_id: Uuid, job_kind: ProcessingJobKind) -> AppResult<()> {
        self.repository.enqueue_job(item_id, job_kind).await?;
        self.repository
            .mark_job_running(item_id, job_kind, &self.worker_id)
            .await?;
        Ok(())
    }

    async fn finish_snapshot_job(
        &self,
        item_id: Uuid,
        outcome: ProcessingOutcome<'_>,
    ) -> AppResult<()> {
        if self
            .repository
            .load_job_state(item_id, ProcessingJobKind::SnapshotThumbnail)
            .await?
            .is_none()
        {
            return Ok(());
        }
        match outcome {
            ProcessingOutcome::Succeeded => {
                self.repository
                    .mark_job_succeeded(item_id, ProcessingJobKind::SnapshotThumbnail)
                    .await?;
            }
            ProcessingOutcome::Failed(error) => {
                self.repository
                    .mark_job_failed(item_id, ProcessingJobKind::SnapshotThumbnail, error)
                    .await?;
            }
        }
        Ok(())
    }
}

enum ProcessingOutcome<'a> {
    Succeeded,
    Failed(&'a str),
}

fn successful_snapshot(
    metadata: ExtractedMetadata,
    thumbnail: Option<StoredThumbnail>,
) -> SnapshotUpdate {
    let (thumbnail_s3_key, thumbnail_content_type) = thumbnail
        .map(|stored| (Some(stored.key), Some(stored.content_type)))
        .unwrap_or((None, None));
    SnapshotUpdate {
        title: metadata.title,
        thumbnail_s3_key,
        thumbnail_content_type,
        author: metadata.author,
        platform: metadata.platform,
        duration_seconds: metadata.duration_seconds,
        archive_status: ArchiveStatus::Succeeded,
        archive_error: None,
    }
}

fn failed_snapshot(metadata: ExtractedMetadata, error: &str) -> SnapshotUpdate {
    SnapshotUpdate {
        title: metadata.title,
        author: metadata.author,
        platform: metadata.platform,
        duration_seconds: metadata.duration_seconds,
        archive_status: ArchiveStatus::Failed,
        archive_error: Some(error.to_string()),
        ..SnapshotUpdate::default()
    }
}

fn failed_enrichment_snapshot(error: &str) -> SnapshotUpdate {
    SnapshotUpdate {
        archive_status: ArchiveStatus::Failed,
        archive_error: Some(error.to_string()),
        ..SnapshotUpdate::default()
    }
}

fn safe_error(error: impl ToString) -> String {
    const MAX_ERROR_LENGTH: usize = 512;
    let mut message = error.to_string();
    message.truncate(MAX_ERROR_LENGTH);
    message
}
