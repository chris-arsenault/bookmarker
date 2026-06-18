use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use aws_sdk_s3::primitives::ByteStream;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedThumbnail {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredThumbnail {
    pub key: String,
    pub content_type: String,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ThumbnailDownloadError {
    #[error("thumbnail download failed: {0}")]
    Failed(String),
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotStoreError {
    #[error(transparent)]
    Download(#[from] ThumbnailDownloadError),
    #[error("snapshot store failed: {0}")]
    Store(String),
}

#[async_trait]
pub trait ThumbnailDownload: Send + Sync {
    async fn download(&self, url: &str) -> Result<DownloadedThumbnail, ThumbnailDownloadError>;
}

#[async_trait]
pub trait ThumbnailStore: Send + Sync {
    async fn store_thumbnail(
        &self,
        item_id: Uuid,
        source_url: &str,
    ) -> Result<StoredThumbnail, SnapshotStoreError>;
}

#[derive(Clone)]
pub struct ReqwestThumbnailDownloader {
    client: reqwest::Client,
}

impl ReqwestThumbnailDownloader {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }
}

#[async_trait]
impl ThumbnailDownload for ReqwestThumbnailDownloader {
    async fn download(&self, url: &str) -> Result<DownloadedThumbnail, ThumbnailDownloadError> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| ThumbnailDownloadError::Failed(err.to_string()))?;
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();
        let bytes = response
            .bytes()
            .await
            .map_err(|err| ThumbnailDownloadError::Failed(err.to_string()))?
            .to_vec();
        Ok(DownloadedThumbnail {
            bytes,
            content_type,
        })
    }
}

pub struct S3ThumbnailStore<D> {
    s3: aws_sdk_s3::Client,
    bucket: String,
    downloader: D,
}

impl<D> S3ThumbnailStore<D> {
    pub fn new(s3: aws_sdk_s3::Client, bucket: impl Into<String>, downloader: D) -> Self {
        Self {
            s3,
            bucket: bucket.into(),
            downloader,
        }
    }
}

#[async_trait]
impl<D> ThumbnailStore for S3ThumbnailStore<D>
where
    D: ThumbnailDownload,
{
    async fn store_thumbnail(
        &self,
        item_id: Uuid,
        source_url: &str,
    ) -> Result<StoredThumbnail, SnapshotStoreError> {
        let thumbnail = self.downloader.download(source_url).await?;
        let key = snapshot_key(item_id, &thumbnail.content_type);
        self.s3
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .content_type(&thumbnail.content_type)
            .body(ByteStream::from(thumbnail.bytes))
            .send()
            .await
            .map_err(|err| SnapshotStoreError::Store(err.to_string()))?;
        Ok(StoredThumbnail {
            key,
            content_type: thumbnail.content_type,
        })
    }
}

pub struct InMemoryThumbnailStore<D> {
    downloader: D,
    objects: Mutex<HashMap<String, Vec<u8>>>,
}

impl<D> InMemoryThumbnailStore<D> {
    pub fn new(downloader: D) -> Self {
        Self {
            downloader,
            objects: Mutex::new(HashMap::new()),
        }
    }

    pub fn stored_bytes(&self, key: &str) -> Option<Vec<u8>> {
        self.objects.lock().unwrap().get(key).cloned()
    }
}

#[async_trait]
impl<D> ThumbnailStore for InMemoryThumbnailStore<D>
where
    D: ThumbnailDownload,
{
    async fn store_thumbnail(
        &self,
        item_id: Uuid,
        source_url: &str,
    ) -> Result<StoredThumbnail, SnapshotStoreError> {
        let thumbnail = self.downloader.download(source_url).await?;
        let key = snapshot_key(item_id, &thumbnail.content_type);
        self.objects
            .lock()
            .unwrap()
            .insert(key.clone(), thumbnail.bytes);
        Ok(StoredThumbnail {
            key,
            content_type: thumbnail.content_type,
        })
    }
}

fn snapshot_key(item_id: Uuid, content_type: &str) -> String {
    format!(
        "snapshots/{item_id}/thumbnail.{}",
        extension_for_content_type(content_type)
    )
}

fn extension_for_content_type(content_type: &str) -> &'static str {
    match content_type.split(';').next().unwrap_or("").trim() {
        "image/jpeg" | "image/jpg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        _ => "bin",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DownloadedThumbnail, InMemoryThumbnailStore, ThumbnailDownload, ThumbnailDownloadError,
        ThumbnailStore,
    };
    use async_trait::async_trait;
    use uuid::Uuid;

    #[tokio::test]
    async fn stores_thumbnail_snapshot_without_hotlink() {
        let item_id = Uuid::parse_str("00000000-0000-0000-0000-000000000505").unwrap();
        let store = InMemoryThumbnailStore::new(FakeDownloader);

        let snapshot = store
            .store_thumbnail(item_id, "https://cdn.example.test/source-thumb.jpg")
            .await
            .unwrap();

        assert_eq!(
            snapshot.key,
            "snapshots/00000000-0000-0000-0000-000000000505/thumbnail.jpg"
        );
        assert_eq!(snapshot.content_type, "image/jpeg");
        assert!(!snapshot.key.contains("cdn.example.test"));
        assert_eq!(
            store.stored_bytes(&snapshot.key).unwrap(),
            b"thumbnail-bytes".to_vec()
        );
    }

    struct FakeDownloader;

    #[async_trait]
    impl ThumbnailDownload for FakeDownloader {
        async fn download(
            &self,
            _url: &str,
        ) -> Result<DownloadedThumbnail, ThumbnailDownloadError> {
            Ok(DownloadedThumbnail {
                bytes: b"thumbnail-bytes".to_vec(),
                content_type: "image/jpeg".to_string(),
            })
        }
    }
}
