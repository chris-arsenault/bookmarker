use std::sync::Arc;

use async_trait::async_trait;
use uuid::Uuid;

use crate::auth::UserContext;
use crate::db::DbPool;
use crate::domain::{ArchiveStatus, InboxStatus, WatchStatus};
use crate::error::{AppError, AppResult};
use crate::library::{
    CaptureItemOutcome, CaptureItemRequest, ItemTag, LibraryItemDetail, LibraryItemSummary,
    LibraryService, LibraryUpdates, ListItemUpdatesQuery, ListItemsQuery, MergeTagsRequest,
    RenameTagRequest, TagCorpusEntry, UpdateItemRequest,
};
use crate::library_pg_capture_helpers::validate_tags;
use crate::url_normalization::{HttpShortUrlResolver, ShortUrlResolver};

#[path = "library_pg_capture.rs"]
mod library_pg_capture;
#[path = "library_pg_capture_insert.rs"]
mod library_pg_capture_insert;
#[path = "library_pg_delete.rs"]
mod library_pg_delete;
#[path = "library_pg_filters.rs"]
mod library_pg_filters;
#[path = "library_pg_rows.rs"]
mod library_pg_rows;
#[path = "library_pg_sql.rs"]
mod library_pg_sql;
#[path = "library_pg_tag_ops.rs"]
mod library_pg_tag_ops;
#[path = "library_pg_updates.rs"]
mod library_pg_updates;

use library_pg_capture_insert::UrlCaptureRows;
use library_pg_filters::PgListFilters;
use library_pg_rows::{ItemRow, ItemTagRow, TagCorpusRow};
use library_pg_sql::*;

#[derive(Clone)]
pub struct PgLibraryService {
    db: DbPool,
    url_resolver: Arc<dyn ShortUrlResolver + Send + Sync>,
}

impl PgLibraryService {
    pub fn new(db: DbPool) -> Self {
        Self::with_url_resolver(db, Arc::new(HttpShortUrlResolver::new()))
    }

    pub fn with_url_resolver(
        db: DbPool,
        url_resolver: Arc<dyn ShortUrlResolver + Send + Sync>,
    ) -> Self {
        Self { db, url_resolver }
    }

    async fn user_id(&self, user: &UserContext) -> AppResult<Option<Uuid>> {
        sqlx::query_scalar(USER_ID_BY_SUB)
            .bind(&user.sub)
            .fetch_optional(&self.db)
            .await
            .map_err(database_error)
    }

    async fn load_tags(&self, item_id: Uuid) -> AppResult<Vec<ItemTag>> {
        let rows: Vec<ItemTagRow> = sqlx::query_as(ITEM_TAGS)
            .bind(item_id)
            .fetch_all(&self.db)
            .await
            .map_err(database_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn detail_from_row(&self, row: ItemRow) -> AppResult<LibraryItemDetail> {
        let tags = self.load_tags(row.id).await?;
        let archive_status = ArchiveStatus::try_from(row.archive_status.as_str())
            .map_err(|err| AppError::Database(err.to_string()))?;
        let watch_status = WatchStatus::try_from(row.watch_status.as_str())
            .map_err(|err| AppError::Database(err.to_string()))?;
        let inbox_status = InboxStatus::try_from(row.inbox_status.as_str())
            .map_err(|err| AppError::Database(err.to_string()))?;
        Ok(row.into_detail(tags, archive_status, watch_status, inbox_status))
    }
}

#[async_trait]
impl LibraryService for PgLibraryService {
    async fn capture_item(
        &self,
        user: &UserContext,
        request: CaptureItemRequest,
    ) -> AppResult<CaptureItemOutcome> {
        library_pg_capture::capture_url(self, user, request).await
    }

    async fn capture_text(
        &self,
        user: &UserContext,
        request: crate::library::CaptureTextRequest,
    ) -> AppResult<CaptureItemOutcome> {
        library_pg_capture::capture_text(self, user, request).await
    }

    async fn list_items(
        &self,
        user: &UserContext,
        query: &ListItemsQuery,
    ) -> AppResult<Vec<LibraryItemSummary>> {
        let Some(user_id) = self.user_id(user).await? else {
            return Ok(Vec::new());
        };
        let filters = PgListFilters::from(query);
        let rows: Vec<ItemRow> =
            sqlx::query_as(&format!("{ITEM_SELECT}{LIST_ITEMS}{LIST_ITEMS_ORDER}"))
                .bind(user_id)
                .bind(filters.platform.as_deref())
                .bind(filters.tag.as_deref())
                .bind(query.created_from)
                .bind(query.created_to)
                .bind(filters.archive_status)
                .bind(filters.watch_status)
                .bind(filters.inbox_status)
                .bind(filters.q.as_deref())
                .fetch_all(&self.db)
                .await
                .map_err(database_error)?;
        let mut items = Vec::with_capacity(rows.len());
        for row in rows {
            items.push(self.detail_from_row(row).await?.summary);
        }
        Ok(items)
    }

    async fn list_item_updates(
        &self,
        user: &UserContext,
        query: &ListItemUpdatesQuery,
    ) -> AppResult<LibraryUpdates> {
        let cursor = time::OffsetDateTime::now_utc();
        let tags = self.list_tag_corpus(user).await?;
        let Some(since) = query.since else {
            return Ok(LibraryUpdates {
                items: Vec::new(),
                deleted_item_ids: Vec::new(),
                tags,
                cursor,
            });
        };
        let Some(user_id) = self.user_id(user).await? else {
            return Ok(LibraryUpdates {
                items: Vec::new(),
                deleted_item_ids: Vec::new(),
                tags,
                cursor,
            });
        };
        let filters = PgListFilters::from(&query.filters);
        let batch = self
            .update_batch(
                user_id,
                &query.filters,
                &filters,
                library_pg_updates::UpdateBatchWindow {
                    since,
                    limit: query.limit,
                    default_cursor: cursor,
                },
            )
            .await?;
        let mut items = Vec::with_capacity(batch.rows.len());
        for row in batch.rows {
            items.push(self.detail_from_row(row).await?.summary);
        }
        Ok(LibraryUpdates {
            items,
            deleted_item_ids: batch.deleted_item_ids,
            tags,
            cursor: batch.cursor,
        })
    }

    async fn get_item(&self, user: &UserContext, item_id: Uuid) -> AppResult<LibraryItemDetail> {
        let user_id = self.required_user_id(user, item_id).await?;
        let row = sqlx::query_as(&format!("{ITEM_SELECT}{GET_ITEM}"))
            .bind(user_id)
            .bind(item_id)
            .fetch_optional(&self.db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(item_id))?;
        self.detail_from_row(row).await
    }

    async fn list_tag_corpus(&self, user: &UserContext) -> AppResult<Vec<TagCorpusEntry>> {
        let Some(user_id) = self.user_id(user).await? else {
            return Ok(Vec::new());
        };
        let rows: Vec<TagCorpusRow> = sqlx::query_as(TAG_CORPUS)
            .bind(user_id)
            .fetch_all(&self.db)
            .await
            .map_err(database_error)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update_item(
        &self,
        user: &UserContext,
        item_id: Uuid,
        request: UpdateItemRequest,
    ) -> AppResult<LibraryItemDetail> {
        let user_id = self.required_user_id(user, item_id).await?;
        let tags = request
            .tags
            .as_ref()
            .map(|tags| validate_tags(tags))
            .transpose()?;
        let mut transaction = self.db.begin().await.map_err(database_error)?;
        let result = sqlx::query(UPDATE_ITEM_ORGANIZATION)
            .bind(item_id)
            .bind(user_id)
            .bind(request.watch_status.map(WatchStatus::as_str))
            .bind(request.inbox_status.map(InboxStatus::as_str))
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?;
        if result.rows_affected() == 0 {
            return Err(not_found(item_id));
        }
        if let Some(notes) = request.notes.as_deref() {
            sqlx::query(UPSERT_ITEM_NOTE)
                .bind(item_id)
                .bind(user_id)
                .bind(notes)
                .execute(&mut *transaction)
                .await
                .map_err(database_error)?;
        }
        if let Some(tags) = tags {
            sqlx::query(DELETE_ITEM_TAGS)
                .bind(item_id)
                .bind(user_id)
                .execute(&mut *transaction)
                .await
                .map_err(database_error)?;
            for tag in tags {
                let tag_id: Uuid = sqlx::query_scalar(UPSERT_TAG)
                    .bind(user_id)
                    .bind(tag.display_name())
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(database_error)?;
                sqlx::query(INSERT_ITEM_TAG)
                    .bind(item_id)
                    .bind(tag_id)
                    .bind(user_id)
                    .execute(&mut *transaction)
                    .await
                    .map_err(database_error)?;
            }
        }
        transaction.commit().await.map_err(database_error)?;
        self.get_item(user, item_id).await
    }

    async fn delete_item(&self, user: &UserContext, item_id: Uuid) -> AppResult<()> {
        library_pg_delete::delete_item(self, user, item_id).await
    }

    async fn rename_tag(
        &self,
        user: &UserContext,
        tag_id: Uuid,
        request: RenameTagRequest,
    ) -> AppResult<Vec<TagCorpusEntry>> {
        let Some(user_id) = self.user_id(user).await? else {
            return Err(library_pg_tag_ops::tag_not_found(tag_id));
        };
        library_pg_tag_ops::rename_tag(&self.db, user_id, tag_id, request).await?;
        self.list_tag_corpus(user).await
    }

    async fn merge_tags(
        &self,
        user: &UserContext,
        source_tag_id: Uuid,
        request: MergeTagsRequest,
    ) -> AppResult<Vec<TagCorpusEntry>> {
        let Some(user_id) = self.user_id(user).await? else {
            return Err(library_pg_tag_ops::tag_not_found(source_tag_id));
        };
        library_pg_tag_ops::merge_tags(&self.db, user_id, source_tag_id, request).await?;
        self.list_tag_corpus(user).await
    }
}

impl PgLibraryService {
    async fn required_user_id(&self, user: &UserContext, item_id: Uuid) -> AppResult<Uuid> {
        self.user_id(user).await?.ok_or_else(|| not_found(item_id))
    }

    async fn upsert_user(&self, user: &UserContext) -> AppResult<Uuid> {
        sqlx::query_scalar(UPSERT_USER)
            .bind(&user.sub)
            .fetch_one(&self.db)
            .await
            .map_err(database_error)
    }

    async fn existing_capture(
        &self,
        user_id: Uuid,
        client_capture_id: Option<&str>,
    ) -> AppResult<Option<LibraryItemDetail>> {
        let Some(client_capture_id) = client_capture_id else {
            return Ok(None);
        };
        let row = sqlx::query_as(&format!("{ITEM_SELECT}{GET_ITEM_BY_CAPTURE_ID}"))
            .bind(user_id)
            .bind(client_capture_id)
            .fetch_optional(&self.db)
            .await
            .map_err(database_error)?;
        match row {
            Some(row) => Ok(Some(self.detail_from_row(row).await?)),
            None => Ok(None),
        }
    }

    async fn existing_canonical_capture(
        &self,
        user_id: Uuid,
        canonical_url: Option<&str>,
    ) -> AppResult<Option<LibraryItemDetail>> {
        let Some(canonical_url) = canonical_url else {
            return Ok(None);
        };
        let row = sqlx::query_as(&format!("{ITEM_SELECT}{GET_ITEM_BY_CANONICAL_URL}"))
            .bind(user_id)
            .bind(canonical_url)
            .fetch_optional(&self.db)
            .await
            .map_err(database_error)?;
        match row {
            Some(row) => Ok(Some(self.detail_from_row(row).await?)),
            None => Ok(None),
        }
    }

    async fn insert_capture(&self, input: UrlCaptureRows<'_>) -> AppResult<Option<Uuid>> {
        library_pg_capture_insert::insert_capture(&self.db, input).await
    }
}

fn database_error(err: sqlx::Error) -> AppError {
    AppError::Database(err.to_string())
}

fn canonical_conflict_without_row() -> AppError {
    AppError::Database("canonical URL conflict did not return an existing item".to_string())
}

fn not_found(item_id: Uuid) -> AppError {
    AppError::NotFound(format!("item {item_id}"))
}

#[cfg(test)]
#[path = "library_pg_tests.rs"]
pub(crate) mod tests;
