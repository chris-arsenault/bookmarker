use serde::{Deserialize, Serialize};
use thiserror::Error;
use time::OffsetDateTime;
use url::Url;
use uuid::Uuid;

#[path = "domain_content.rs"]
mod domain_content;

pub use domain_content::{ImageUploadStatus, ItemKind, TextSnippetBody};

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum DomainError {
    #[error("submitted URL is empty")]
    EmptyUrl,
    #[error("submitted URL must be an absolute URL")]
    InvalidUrl,
    #[error("submitted URL scheme is not supported: {0}")]
    UnsupportedUrlScheme(String),
    #[error("text snippet is empty")]
    EmptyTextSnippet,
    #[error("tag name is empty")]
    EmptyTagName,
    #[error("invalid {field}: {value}")]
    InvalidValue { field: &'static str, value: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmittedUrl(String);

impl SubmittedUrl {
    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let submitted = value.into();
        if submitted.trim().is_empty() {
            return Err(DomainError::EmptyUrl);
        }

        let parsed = Url::parse(&submitted).map_err(|_| DomainError::InvalidUrl)?;
        match parsed.scheme() {
            "http" | "https" => Ok(Self(submitted)),
            scheme => Err(DomainError::UnsupportedUrlScheme(scheme.to_string())),
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagName {
    display_name: String,
    normalized_name: String,
}

impl TagName {
    pub fn new(value: impl AsRef<str>) -> Result<Self, DomainError> {
        let display_name = value.as_ref().trim();
        if display_name.is_empty() {
            return Err(DomainError::EmptyTagName);
        }

        Ok(Self {
            display_name: display_name.to_string(),
            normalized_name: display_name.to_lowercase(),
        })
    }

    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    pub fn normalized_name(&self) -> &str {
        &self.normalized_name
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveStatus {
    Pending,
    Succeeded,
    Failed,
    NotApplicable,
}

impl ArchiveStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::NotApplicable => "not_applicable",
        }
    }
}

impl TryFrom<&str> for ArchiveStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "not_applicable" => Ok(Self::NotApplicable),
            _ => Err(invalid_value("archive_status", value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatchStatus {
    Unwatched,
    Watched,
}

impl WatchStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unwatched => "unwatched",
            Self::Watched => "watched",
        }
    }
}

impl TryFrom<&str> for WatchStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "unwatched" => Ok(Self::Unwatched),
            "watched" => Ok(Self::Watched),
            _ => Err(invalid_value("watch_status", value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InboxStatus {
    Unsorted,
    Organized,
}

impl InboxStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unsorted => "unsorted",
            Self::Organized => "organized",
        }
    }
}

impl TryFrom<&str> for InboxStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "unsorted" => Ok(Self::Unsorted),
            "organized" => Ok(Self::Organized),
            _ => Err(invalid_value("inbox_status", value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingJobKind {
    NormalizeUrl,
    EnrichMetadata,
    SnapshotThumbnail,
}

impl ProcessingJobKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::NormalizeUrl => "normalize_url",
            Self::EnrichMetadata => "enrich_metadata",
            Self::SnapshotThumbnail => "snapshot_thumbnail",
        }
    }
}

impl TryFrom<&str> for ProcessingJobKind {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "normalize_url" => Ok(Self::NormalizeUrl),
            "enrich_metadata" => Ok(Self::EnrichMetadata),
            "snapshot_thumbnail" => Ok(Self::SnapshotThumbnail),
            _ => Err(invalid_value("job_kind", value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProcessingStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
}

impl ProcessingStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        }
    }
}

impl TryFrom<&str> for ProcessingStatus {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "queued" => Ok(Self::Queued),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            _ => Err(invalid_value("processing_status", value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub watch_status: WatchStatus,
    pub inbox_status: InboxStatus,
    pub created_at: OffsetDateTime,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemUrlRecord {
    pub item_id: Uuid,
    pub original_url: String,
    pub canonical_url: Option<String>,
    pub normalization_status: ArchiveStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub display_name: String,
    pub normalized_name: String,
    pub usage_count: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemNoteRecord {
    pub item_id: Uuid,
    pub body: String,
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetadataSnapshotRecord {
    pub item_id: Uuid,
    pub title: Option<String>,
    pub thumbnail_s3_key: Option<String>,
    pub author: Option<String>,
    pub platform: Option<String>,
    pub duration_seconds: Option<i32>,
    pub archive_status: ArchiveStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessingJobRecord {
    pub id: Uuid,
    pub item_id: Uuid,
    pub job_kind: ProcessingJobKind,
    pub status: ProcessingStatus,
    pub attempt_count: i32,
    pub idempotency_key: String,
}

fn invalid_value(field: &'static str, value: &str) -> DomainError {
    DomainError::InvalidValue {
        field,
        value: value.to_string(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::{
        ArchiveStatus, InboxStatus, ProcessingJobKind, ProcessingStatus, SubmittedUrl, TagName,
        WatchStatus,
    };

    #[test]
    fn domain_status_values_match_database_contract() {
        assert_archive_status_values();
        assert_watch_status_values();
        assert_inbox_status_values();
        assert_processing_job_kind_values();
        assert_processing_status_values();
    }

    fn assert_archive_status_values() {
        assert_eq!(ArchiveStatus::Pending.as_str(), "pending");
        assert_eq!(ArchiveStatus::Succeeded.as_str(), "succeeded");
        assert_eq!(ArchiveStatus::Failed.as_str(), "failed");
        assert!(ArchiveStatus::try_from("archived").is_err());
    }

    fn assert_watch_status_values() {
        assert_eq!(WatchStatus::Unwatched.as_str(), "unwatched");
        assert_eq!(WatchStatus::Watched.as_str(), "watched");
    }

    fn assert_inbox_status_values() {
        assert_eq!(InboxStatus::Unsorted.as_str(), "unsorted");
        assert_eq!(InboxStatus::Organized.as_str(), "organized");
        assert!(InboxStatus::try_from("hidden").is_err());
    }

    fn assert_processing_job_kind_values() {
        assert_eq!(ProcessingJobKind::NormalizeUrl.as_str(), "normalize_url");
        assert_eq!(
            ProcessingJobKind::EnrichMetadata.as_str(),
            "enrich_metadata"
        );
        assert_eq!(
            ProcessingJobKind::SnapshotThumbnail.as_str(),
            "snapshot_thumbnail"
        );
    }

    fn assert_processing_status_values() {
        assert_eq!(ProcessingStatus::Queued.as_str(), "queued");
        assert_eq!(ProcessingStatus::Running.as_str(), "running");
        assert_eq!(ProcessingStatus::Succeeded.as_str(), "succeeded");
        assert_eq!(ProcessingStatus::Failed.as_str(), "failed");
        assert!(ProcessingStatus::try_from("pending").is_err());
    }

    #[test]
    fn submitted_url_accepts_http_and_https_without_normalizing() {
        let url =
            SubmittedUrl::new("https://youtu.be/video-id?utm_source=share&keep=value").unwrap();

        assert_eq!(
            url.as_str(),
            "https://youtu.be/video-id?utm_source=share&keep=value"
        );
        assert!(SubmittedUrl::new("http://example.com/path").is_ok());
        assert!(SubmittedUrl::new("").is_err());
        assert!(SubmittedUrl::new("/relative/path").is_err());
        assert!(SubmittedUrl::new("ftp://example.com/file").is_err());
    }

    #[test]
    fn tag_name_trims_display_and_builds_explicit_corpus_key() {
        let tag = TagName::new("  Watch Later  ").unwrap();

        assert_eq!(tag.display_name(), "Watch Later");
        assert_eq!(tag.normalized_name(), "watch later");
        assert!(TagName::new("   ").is_err());
    }
}
