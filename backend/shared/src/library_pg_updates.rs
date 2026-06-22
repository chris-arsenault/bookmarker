use uuid::Uuid;

use super::{
    database_error,
    library_pg_filters::PgListFilters,
    library_pg_rows::ItemRow,
    library_pg_sql::{ITEM_SELECT, LIST_ITEM_DELETIONS, LIST_ITEM_UPDATES},
    PgLibraryService,
};
use crate::error::AppResult;
use crate::library::ListItemsQuery;

pub(super) struct ItemUpdateBatch {
    pub(super) rows: Vec<ItemRow>,
    pub(super) deleted_item_ids: Vec<Uuid>,
    pub(super) cursor: time::OffsetDateTime,
}

pub(super) struct UpdateBatchWindow {
    pub(super) since: time::OffsetDateTime,
    pub(super) limit: i64,
    pub(super) default_cursor: time::OffsetDateTime,
}

#[derive(sqlx::FromRow)]
struct DeletedItemRow {
    item_id: Uuid,
    deleted_at: time::OffsetDateTime,
}

impl PgLibraryService {
    pub(super) async fn update_batch(
        &self,
        user_id: Uuid,
        query: &ListItemsQuery,
        filters: &PgListFilters,
        window: UpdateBatchWindow,
    ) -> AppResult<ItemUpdateBatch> {
        let rows = self
            .updated_rows(user_id, query, filters, window.since, window.limit)
            .await?;
        let deleted_rows = self
            .deleted_rows(user_id, window.since, window.limit)
            .await?;
        Ok(ItemUpdateBatch {
            cursor: batch_cursor(window.default_cursor, window.limit, &rows, &deleted_rows),
            deleted_item_ids: deleted_rows.into_iter().map(|row| row.item_id).collect(),
            rows,
        })
    }

    async fn updated_rows(
        &self,
        user_id: Uuid,
        query: &ListItemsQuery,
        filters: &PgListFilters,
        since: time::OffsetDateTime,
        limit: i64,
    ) -> AppResult<Vec<ItemRow>> {
        sqlx::query_as(&format!("{ITEM_SELECT}{LIST_ITEM_UPDATES}"))
            .bind(user_id)
            .bind(filters.platform.as_deref())
            .bind(filters.tag.as_deref())
            .bind(query.created_from)
            .bind(query.created_to)
            .bind(filters.archive_status)
            .bind(filters.watch_status)
            .bind(filters.inbox_status)
            .bind(filters.q.as_deref())
            .bind(since)
            .bind(limit)
            .fetch_all(&self.db)
            .await
            .map_err(database_error)
    }

    async fn deleted_rows(
        &self,
        user_id: Uuid,
        since: time::OffsetDateTime,
        limit: i64,
    ) -> AppResult<Vec<DeletedItemRow>> {
        sqlx::query_as(LIST_ITEM_DELETIONS)
            .bind(user_id)
            .bind(since)
            .bind(limit)
            .fetch_all(&self.db)
            .await
            .map_err(database_error)
    }
}

fn batch_cursor(
    default_cursor: time::OffsetDateTime,
    limit: i64,
    rows: &[ItemRow],
    deleted_rows: &[DeletedItemRow],
) -> time::OffsetDateTime {
    saturated_cursors(limit, rows, deleted_rows)
        .into_iter()
        .min()
        .unwrap_or(default_cursor)
}

fn saturated_cursors(
    limit: i64,
    rows: &[ItemRow],
    deleted_rows: &[DeletedItemRow],
) -> Vec<time::OffsetDateTime> {
    let limit = limit as usize;
    let mut cursors = Vec::new();
    if rows.len() >= limit {
        cursors.extend(rows.last().map(|row| row.update_cursor));
    }
    if deleted_rows.len() >= limit {
        cursors.extend(deleted_rows.last().map(|row| row.deleted_at));
    }
    cursors
}
