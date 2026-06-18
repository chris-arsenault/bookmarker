use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use shared::config::ConfigError;
use shared::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumbnailObject {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

#[async_trait]
pub trait ThumbnailReader: Send + Sync {
    async fn read_thumbnail(&self, key: &str) -> AppResult<ThumbnailObject>;
}

pub struct S3ThumbnailReader {
    s3: aws_sdk_s3::Client,
    bucket: String,
}

impl S3ThumbnailReader {
    pub async fn from_env() -> AppResult<Self> {
        let bucket = snapshot_bucket()?;
        let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
        Ok(Self {
            s3: aws_sdk_s3::Client::new(&config),
            bucket,
        })
    }
}

#[async_trait]
impl ThumbnailReader for S3ThumbnailReader {
    async fn read_thumbnail(&self, key: &str) -> AppResult<ThumbnailObject> {
        let output = self
            .s3
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|err| external_error(err.to_string()))?;
        let content_type = output
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();
        let bytes = output
            .body
            .collect()
            .await
            .map_err(|err| external_error(err.to_string()))?
            .into_bytes()
            .to_vec();
        Ok(ThumbnailObject {
            bytes,
            content_type,
        })
    }
}

#[derive(Default)]
pub struct InMemoryThumbnailReader {
    objects: Arc<HashMap<String, ThumbnailObject>>,
}

impl InMemoryThumbnailReader {
    pub fn from_objects(
        objects: impl IntoIterator<Item = (impl Into<String>, ThumbnailObject)>,
    ) -> Self {
        Self {
            objects: Arc::new(
                objects
                    .into_iter()
                    .map(|(key, object)| (key.into(), object))
                    .collect(),
            ),
        }
    }
}

#[async_trait]
impl ThumbnailReader for InMemoryThumbnailReader {
    async fn read_thumbnail(&self, key: &str) -> AppResult<ThumbnailObject> {
        self.objects
            .get(key)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("thumbnail {key}")))
    }
}

fn snapshot_bucket() -> AppResult<String> {
    std::env::var("SNAPSHOT_BUCKET")
        .ok()
        .map(|bucket| bucket.trim().to_string())
        .filter(|bucket| !bucket.is_empty())
        .ok_or(AppError::Config(ConfigError::MissingEnv {
            name: "SNAPSHOT_BUCKET",
        }))
}

fn external_error(message: String) -> AppError {
    AppError::ExternalService {
        service: "s3",
        message,
    }
}
