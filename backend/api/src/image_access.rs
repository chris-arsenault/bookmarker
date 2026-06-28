use std::collections::HashMap;
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
pub struct ImageAccessTarget {
    pub view_url: String,
    pub download_url: String,
    pub expires_in_seconds: u64,
}

#[async_trait]
pub trait ImageObjectStore: Send + Sync {
    async fn upload_target(&self, key: &str, content_type: &str) -> AppResult<ImageUploadTarget>;
    async fn access_target(
        &self,
        key: &str,
        content_type: &str,
        download_name: &str,
    ) -> AppResult<ImageAccessTarget>;
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
        let config = presigning_config(15 * 60)?;
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

    async fn access_target(
        &self,
        key: &str,
        content_type: &str,
        download_name: &str,
    ) -> AppResult<ImageAccessTarget> {
        let expires_in_seconds = 10 * 60;
        let view_url = self.presigned_get(key, content_type, None).await?;
        let disposition = content_disposition(download_name);
        let download_url = self
            .presigned_get(key, content_type, Some(&disposition))
            .await?;
        Ok(ImageAccessTarget {
            view_url,
            download_url,
            expires_in_seconds,
        })
    }
}

impl S3ImageObjectStore {
    async fn presigned_get(
        &self,
        key: &str,
        content_type: &str,
        content_disposition: Option<&str>,
    ) -> AppResult<String> {
        let mut request = self
            .s3
            .get_object()
            .bucket(&self.bucket)
            .key(key)
            .response_content_type(content_type);
        if let Some(value) = content_disposition {
            request = request.response_content_disposition(value);
        }
        let presigned = request
            .presigned(presigning_config(10 * 60)?)
            .await
            .map_err(|err| external_error(err.to_string()))?;
        Ok(presigned.uri().to_string())
    }
}

#[derive(Default)]
pub struct InMemoryImageObjectStore;

#[async_trait]
impl ImageObjectStore for InMemoryImageObjectStore {
    async fn upload_target(&self, key: &str, content_type: &str) -> AppResult<ImageUploadTarget> {
        Ok(ImageUploadTarget {
            url: format!("https://upload.example.test/{key}"),
            headers: upload_headers(content_type),
        })
    }

    async fn access_target(
        &self,
        key: &str,
        _content_type: &str,
        download_name: &str,
    ) -> AppResult<ImageAccessTarget> {
        Ok(ImageAccessTarget {
            view_url: format!("https://download.example.test/{key}"),
            download_url: format!("https://download.example.test/{key}?download={download_name}"),
            expires_in_seconds: 600,
        })
    }
}

fn presigning_config(expires_in_seconds: u64) -> AppResult<PresigningConfig> {
    PresigningConfig::expires_in(Duration::from_secs(expires_in_seconds))
        .map_err(|err| external_error(err.to_string()))
}

fn content_disposition(download_name: &str) -> String {
    format!(
        "attachment; filename=\"{}\"",
        download_name.replace(['\\', '"', '\r', '\n'], "_")
    )
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
