use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_s3::presigning::PresigningConfig;
use serde::Serialize;
use shared::config::ConfigError;
use shared::error::{AppError, AppResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ImageUploadTarget {
    pub url: String,
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageObject {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

#[async_trait]
pub trait ImageObjectStore: Send + Sync {
    async fn upload_target(&self, key: &str, content_type: &str) -> AppResult<ImageUploadTarget>;
    async fn read_image(&self, key: &str) -> AppResult<ImageObject>;
}

pub struct S3ImageObjectStore {
    s3: aws_sdk_s3::Client,
    bucket: String,
}

impl S3ImageObjectStore {
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
impl ImageObjectStore for S3ImageObjectStore {
    async fn upload_target(&self, key: &str, content_type: &str) -> AppResult<ImageUploadTarget> {
        let config = PresigningConfig::expires_in(Duration::from_secs(15 * 60))
            .map_err(|err| external_error(err.to_string()))?;
        let presigned = self
            .s3
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(config)
            .await
            .map_err(|err| external_error(err.to_string()))?;
        Ok(ImageUploadTarget {
            url: presigned.uri().to_string(),
            headers: upload_headers(content_type),
        })
    }

    async fn read_image(&self, key: &str) -> AppResult<ImageObject> {
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
        Ok(ImageObject {
            bytes,
            content_type,
        })
    }
}

#[derive(Default)]
pub struct InMemoryImageObjectStore {
    objects: Arc<HashMap<String, ImageObject>>,
}

impl InMemoryImageObjectStore {
    pub fn from_objects(
        objects: impl IntoIterator<Item = (impl Into<String>, ImageObject)>,
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
impl ImageObjectStore for InMemoryImageObjectStore {
    async fn upload_target(&self, key: &str, content_type: &str) -> AppResult<ImageUploadTarget> {
        Ok(ImageUploadTarget {
            url: format!("https://upload.example.test/{key}"),
            headers: upload_headers(content_type),
        })
    }

    async fn read_image(&self, key: &str) -> AppResult<ImageObject> {
        self.objects
            .get(key)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("image {key}")))
    }
}

fn upload_headers(content_type: &str) -> HashMap<String, String> {
    HashMap::from([("content-type".to_string(), content_type.to_string())])
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
