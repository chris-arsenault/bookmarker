use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::UserContext;
use crate::domain::{ArchiveStatus, InboxStatus, ItemKind, WatchStatus};
use crate::error::{AppError, AppResult};

#[path = "library_in_memory.rs"]
mod library_in_memory;
#[path = "library_query.rs"]
mod library_query;

pub use library_in_memory::InMemoryLibraryService;

#[async_trait]
pub trait LibraryService: Send + Sync {
    async fn capture_item(
        &self,
        _user: &UserContext,
        _request: CaptureItemRequest,
    ) -> AppResult<CaptureItemOutcome> {
        Err(AppError::Internal("capture is not implemented".to_string()))
    }

    async fn capture_text(
        &self,
        _user: &UserContext,
        _request: CaptureTextRequest,
    ) -> AppResult<CaptureItemOutcome> {
        Err(AppError::Internal(
            "text capture is not implemented".to_string(),
        ))
    }

    async fn list_items(
        &self,
        user: &UserContext,
        query: &ListItemsQuery,
    ) -> AppResult<Vec<LibraryItemSummary>>;

    async fn list_item_updates(
        &self,
        _user: &UserContext,
        _query: &ListItemUpdatesQuery,
    ) -> AppResult<LibraryUpdates> {
        Err(AppError::Internal(
            "item updates are not implemented".to_string(),
        ))
    }

    async fn get_item(&self, user: &UserContext, item_id: Uuid) -> AppResult<LibraryItemDetail>;

    async fn list_tag_corpus(&self, user: &UserContext) -> AppResult<Vec<TagCorpusEntry>>;

    async fn update_item(
        &self,
        user: &UserContext,
        item_id: Uuid,
        request: UpdateItemRequest,
    ) -> AppResult<LibraryItemDetail>;

    async fn delete_item(&self, user: &UserContext, item_id: Uuid) -> AppResult<()>;

    async fn rename_tag(
        &self,
        _user: &UserContext,
        _tag_id: Uuid,
        _request: RenameTagRequest,
    ) -> AppResult<Vec<TagCorpusEntry>> {
        Err(AppError::Internal(
            "tag rename is not implemented".to_string(),
        ))
    }

    async fn merge_tags(
        &self,
        _user: &UserContext,
        _source_tag_id: Uuid,
        _request: MergeTagsRequest,
    ) -> AppResult<Vec<TagCorpusEntry>> {
        Err(AppError::Internal(
            "tag merge is not implemented".to_string(),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureItemRequest {
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub client_capture_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureTextRequest {
    pub plain_text: String,
    pub html: Option<String>,
    pub source_app: Option<String>,
    pub source_device: Option<String>,
    pub capture_method: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub client_capture_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureItemOutcome {
    pub item: LibraryItemDetail,
    pub created: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItemsQuery {
    pub platform: Option<String>,
    pub tag: Option<String>,
    pub created_from: Option<OffsetDateTime>,
    pub created_to: Option<OffsetDateTime>,
    pub archive_status: Option<ArchiveStatus>,
    pub watch_status: Option<WatchStatus>,
    pub inbox_status: Option<InboxStatus>,
    pub q: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItemUpdatesQuery {
    pub since: Option<OffsetDateTime>,
    pub limit: i64,
    pub filters: ListItemsQuery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryUpdates {
    pub items: Vec<LibraryItemSummary>,
    pub deleted_item_ids: Vec<Uuid>,
    pub tags: Vec<TagCorpusEntry>,
    pub cursor: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryItemSummary {
    pub id: Uuid,
    pub item_kind: ItemKind,
    pub url: Option<ItemUrlSummary>,
    pub text: Option<ItemTextSummary>,
    pub title: Option<String>,
    pub thumbnail_s3_key: Option<String>,
    pub author: Option<String>,
    pub platform: Option<String>,
    pub duration_seconds: Option<i32>,
    pub archive_status: ArchiveStatus,
    pub watch_status: WatchStatus,
    pub inbox_status: InboxStatus,
    pub tags: Vec<ItemTag>,
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemUrlSummary {
    pub original_url: String,
    pub canonical_url: Option<String>,
    pub copy_url: String,
}

impl ItemUrlSummary {
    pub fn new(original_url: String, canonical_url: Option<String>) -> Self {
        let copy_url = Self::copy_url_for(&original_url, canonical_url.as_deref());
        Self {
            original_url,
            canonical_url,
            copy_url,
        }
    }

    pub fn copy_url_for(original_url: &str, canonical_url: Option<&str>) -> String {
        canonical_url.unwrap_or(original_url).to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemTextSummary {
    pub plain_text: String,
    pub preview: String,
    pub content_hash: String,
    pub html: Option<String>,
    pub source_app: Option<String>,
    pub source_device: Option<String>,
    pub capture_method: String,
}

impl ItemTextSummary {
    pub fn new(
        plain_text: String,
        html: Option<String>,
        content_hash: String,
        source_app: Option<String>,
        source_device: Option<String>,
        capture_method: String,
    ) -> Self {
        let preview = text_preview(&plain_text);
        Self {
            plain_text,
            preview,
            content_hash,
            html,
            source_app,
            source_device,
            capture_method,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryItemDetail {
    pub summary: LibraryItemSummary,
    pub notes: String,
}

pub fn text_preview(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_PREVIEW_LEN: usize = 120;
    if normalized.chars().count() <= MAX_PREVIEW_LEN {
        return normalized;
    }
    let mut preview = normalized
        .chars()
        .take(MAX_PREVIEW_LEN - 3)
        .collect::<String>();
    preview.push_str("...");
    preview
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemTag {
    pub id: Uuid,
    pub display_name: String,
    pub normalized_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagCorpusEntry {
    pub id: Uuid,
    pub display_name: String,
    pub normalized_name: String,
    pub usage_count: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateItemRequest {
    pub watch_status: Option<WatchStatus>,
    pub inbox_status: Option<InboxStatus>,
    pub notes: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenameTagRequest {
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeTagsRequest {
    pub target_tag_id: Uuid,
}

#[cfg(test)]
#[path = "library_tests.rs"]
pub(crate) mod tests;
