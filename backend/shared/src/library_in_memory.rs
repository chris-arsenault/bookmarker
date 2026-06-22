use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::UserContext;
use crate::domain::{ArchiveStatus, InboxStatus, ItemKind, SubmittedUrl, WatchStatus};
use crate::error::{AppError, AppResult};
use crate::url_normalization::{
    normalize_url_with_resolver, HttpShortUrlResolver, NormalizedUrl, ShortUrlResolver,
};

use super::library_query::item_matches_query;
use super::{
    CaptureItemOutcome, CaptureItemRequest, CaptureTextRequest, ItemTag, ItemUrlSummary,
    LibraryItemDetail, LibraryItemSummary, LibraryService, LibraryUpdates, ListItemUpdatesQuery,
    ListItemsQuery, MergeTagsRequest, RenameTagRequest, TagCorpusEntry, UpdateItemRequest,
};

pub struct InMemoryLibraryService {
    items_by_user: Arc<Mutex<UserItems>>,
    tags_by_user: Arc<Mutex<UserTags>>,
    capture_ids_by_user: Arc<Mutex<CaptureIdsByUser>>,
    url_resolver: Arc<dyn ShortUrlResolver + Send + Sync>,
}

type UserItems = HashMap<String, Vec<LibraryItemDetail>>;
type UserTags = HashMap<String, Vec<TagCorpusEntry>>;
type CaptureIdsByUser = HashMap<String, HashMap<String, Uuid>>;

#[path = "library_in_memory_delete.rs"]
mod library_in_memory_delete;
#[path = "library_in_memory_text.rs"]
mod library_in_memory_text;
#[path = "library_in_memory_update.rs"]
mod library_in_memory_update;
#[path = "library_in_memory_tag_ops.rs"]
mod tag_ops;

impl Default for InMemoryLibraryService {
    fn default() -> Self {
        Self {
            items_by_user: Arc::new(Mutex::new(UserItems::default())),
            tags_by_user: Arc::new(Mutex::new(UserTags::default())),
            capture_ids_by_user: Arc::new(Mutex::new(CaptureIdsByUser::default())),
            url_resolver: Arc::new(HttpShortUrlResolver::new()),
        }
    }
}

impl InMemoryLibraryService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_user_items(
        user_sub: impl Into<String>,
        items: impl IntoIterator<Item = LibraryItemDetail>,
    ) -> Self {
        let service = Self::new();
        service
            .items_by_user
            .lock()
            .unwrap()
            .insert(user_sub.into(), items.into_iter().collect());
        service
    }

    pub fn set_user_tags(
        &self,
        user_sub: impl Into<String>,
        tags: impl IntoIterator<Item = TagCorpusEntry>,
    ) {
        self.tags_by_user
            .lock()
            .unwrap()
            .insert(user_sub.into(), tags.into_iter().collect());
    }
}

#[async_trait]
impl LibraryService for InMemoryLibraryService {
    async fn capture_item(
        &self,
        user: &UserContext,
        request: CaptureItemRequest,
    ) -> AppResult<CaptureItemOutcome> {
        let CaptureItemRequest {
            url,
            title,
            tags,
            client_capture_id,
        } = request;
        let original_url = SubmittedUrl::new(url)
            .map_err(validation_error)?
            .into_string();
        let client_capture_id = validate_client_capture_id(client_capture_id)?;
        if let Some(item) = self.existing_capture(user, client_capture_id.as_deref())? {
            return Ok(CaptureItemOutcome {
                item,
                created: false,
            });
        }

        let normalized_url =
            normalize_url_with_resolver(&original_url, self.url_resolver.as_ref()).await;
        if let Some(item) =
            self.existing_canonical_capture(user, normalized_url.canonical_url.as_deref())
        {
            return Ok(CaptureItemOutcome {
                item,
                created: false,
            });
        }

        let item = self.new_capture_item(
            original_url,
            normalized_url,
            clean_optional(title),
            tag_ops::capture_tags(&self.tags_by_user, &user.sub, &tags)?,
        );
        self.store_capture(user, client_capture_id, item.clone());
        Ok(CaptureItemOutcome {
            item,
            created: true,
        })
    }

    async fn capture_text(
        &self,
        user: &UserContext,
        request: CaptureTextRequest,
    ) -> AppResult<CaptureItemOutcome> {
        library_in_memory_text::capture_text(self, user, request).await
    }

    async fn list_items(
        &self,
        user: &UserContext,
        query: &ListItemsQuery,
    ) -> AppResult<Vec<LibraryItemSummary>> {
        let items = self
            .items_by_user
            .lock()
            .unwrap()
            .get(&user.sub)
            .cloned()
            .unwrap_or_default();
        Ok(items
            .into_iter()
            .filter(|item| item_matches_query(item, query))
            .map(|item| item.summary)
            .collect())
    }

    async fn list_item_updates(
        &self,
        user: &UserContext,
        query: &ListItemUpdatesQuery,
    ) -> AppResult<LibraryUpdates> {
        let cursor = OffsetDateTime::now_utc();
        let tags = self.list_tag_corpus(user).await?;
        let Some(since) = query.since else {
            return Ok(LibraryUpdates {
                items: Vec::new(),
                deleted_item_ids: Vec::new(),
                tags,
                cursor,
            });
        };
        let items: Vec<LibraryItemSummary> = self
            .items_by_user
            .lock()
            .unwrap()
            .get(&user.sub)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|item| item.summary.created_at > since)
            .filter(|item| item_matches_query(item, &query.filters))
            .take(query.limit as usize)
            .map(|item| item.summary)
            .collect();
        let cursor = library_in_memory_update::limited_cursor(cursor, query.limit, &items);
        Ok(LibraryUpdates {
            items,
            deleted_item_ids: Vec::new(),
            tags,
            cursor,
        })
    }

    async fn get_item(&self, user: &UserContext, item_id: Uuid) -> AppResult<LibraryItemDetail> {
        self.find_item(user, item_id)
    }

    async fn list_tag_corpus(&self, user: &UserContext) -> AppResult<Vec<TagCorpusEntry>> {
        Ok(self
            .tags_by_user
            .lock()
            .unwrap()
            .get(&user.sub)
            .cloned()
            .unwrap_or_default())
    }

    async fn update_item(
        &self,
        user: &UserContext,
        item_id: Uuid,
        request: UpdateItemRequest,
    ) -> AppResult<LibraryItemDetail> {
        library_in_memory_update::update_item(self, user, item_id, request)
    }

    async fn delete_item(&self, user: &UserContext, item_id: Uuid) -> AppResult<()> {
        library_in_memory_delete::delete_item(self, user, item_id)
    }

    async fn rename_tag(
        &self,
        user: &UserContext,
        tag_id: Uuid,
        request: RenameTagRequest,
    ) -> AppResult<Vec<TagCorpusEntry>> {
        tag_ops::rename_tag(
            &self.items_by_user,
            &self.tags_by_user,
            &user.sub,
            tag_id,
            request,
        )
    }

    async fn merge_tags(
        &self,
        user: &UserContext,
        source_tag_id: Uuid,
        request: MergeTagsRequest,
    ) -> AppResult<Vec<TagCorpusEntry>> {
        tag_ops::merge_tags(
            &self.items_by_user,
            &self.tags_by_user,
            &user.sub,
            source_tag_id,
            request,
        )
    }
}

impl InMemoryLibraryService {
    fn existing_capture(
        &self,
        user: &UserContext,
        client_capture_id: Option<&str>,
    ) -> AppResult<Option<LibraryItemDetail>> {
        let Some(client_capture_id) = client_capture_id else {
            return Ok(None);
        };
        let item_id = self
            .capture_ids_by_user
            .lock()
            .unwrap()
            .get(&user.sub)
            .and_then(|user_ids| user_ids.get(client_capture_id))
            .copied();
        item_id
            .map(|item_id| self.find_item(user, item_id))
            .transpose()
    }

    fn existing_canonical_capture(
        &self,
        user: &UserContext,
        canonical_url: Option<&str>,
    ) -> Option<LibraryItemDetail> {
        let canonical_url = canonical_url?;
        self.items_by_user
            .lock()
            .unwrap()
            .get(&user.sub)
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| {
                        item.summary
                            .url
                            .as_ref()
                            .and_then(|url| url.canonical_url.as_deref())
                            == Some(canonical_url)
                    })
                    .cloned()
            })
    }

    fn new_capture_item(
        &self,
        original_url: String,
        normalized_url: NormalizedUrl,
        title: Option<String>,
        tags: Vec<ItemTag>,
    ) -> LibraryItemDetail {
        let NormalizedUrl { canonical_url, .. } = normalized_url;
        LibraryItemDetail {
            summary: LibraryItemSummary {
                id: Uuid::new_v4(),
                item_kind: ItemKind::Url,
                url: Some(ItemUrlSummary::new(original_url, canonical_url)),
                text: None,
                title,
                thumbnail_s3_key: None,
                author: None,
                platform: None,
                duration_seconds: None,
                archive_status: ArchiveStatus::Pending,
                watch_status: WatchStatus::Unwatched,
                inbox_status: InboxStatus::Unsorted,
                tags,
                created_at: OffsetDateTime::now_utc(),
            },
            notes: String::new(),
        }
    }

    fn store_capture(
        &self,
        user: &UserContext,
        client_capture_id: Option<String>,
        item: LibraryItemDetail,
    ) {
        self.items_by_user
            .lock()
            .unwrap()
            .entry(user.sub.clone())
            .or_default()
            .push(item.clone());
        if let Some(client_capture_id) = client_capture_id {
            self.capture_ids_by_user
                .lock()
                .unwrap()
                .entry(user.sub.clone())
                .or_default()
                .insert(client_capture_id, item.summary.id);
        }
    }

    fn find_item(&self, user: &UserContext, item_id: Uuid) -> AppResult<LibraryItemDetail> {
        self.items_by_user
            .lock()
            .unwrap()
            .get(&user.sub)
            .and_then(|items| {
                items
                    .iter()
                    .find(|item| item.summary.id == item_id)
                    .cloned()
            })
            .ok_or_else(|| not_found(item_id))
    }
}

fn validate_client_capture_id(value: Option<String>) -> AppResult<Option<String>> {
    value
        .map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Err(AppError::Validation(
                    "client_capture_id must not be empty".to_string(),
                ))
            } else {
                Ok(trimmed.to_string())
            }
        })
        .transpose()
}

fn validation_error(err: impl ToString) -> AppError {
    AppError::Validation(err.to_string())
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn not_found(item_id: Uuid) -> AppError {
    AppError::NotFound(format!("item {item_id}"))
}
