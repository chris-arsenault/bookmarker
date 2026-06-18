use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::db::DbPool;
use crate::domain::{ItemKind, TagName};
use crate::error::AppResult;
use crate::url_normalization::NormalizedUrl;

use super::{
    database_error, INSERT_CAPTURE_ITEM, INSERT_CAPTURE_TITLE, INSERT_CAPTURE_URL, INSERT_ITEM_TAG,
    UPSERT_TAG,
};

pub(super) struct UrlCaptureRows<'a> {
    pub(super) user_id: Uuid,
    pub(super) original_url: &'a str,
    pub(super) normalized_url: &'a NormalizedUrl,
    pub(super) client_capture_id: Option<&'a str>,
    pub(super) title: Option<&'a str>,
    pub(super) tags: &'a [TagName],
}

pub(super) async fn insert_capture(
    db: &DbPool,
    input: UrlCaptureRows<'_>,
) -> AppResult<Option<Uuid>> {
    let mut transaction = db.begin().await.map_err(database_error)?;
    let item_id = insert_capture_item(&mut transaction, &input).await?;
    if !insert_capture_url(&mut transaction, item_id, &input).await? {
        transaction.rollback().await.map_err(database_error)?;
        return Ok(None);
    }
    insert_capture_title(&mut transaction, item_id, input.user_id, input.title).await?;
    insert_capture_tags(&mut transaction, item_id, input.user_id, input.tags).await?;
    transaction.commit().await.map_err(database_error)?;
    Ok(Some(item_id))
}

async fn insert_capture_item(
    transaction: &mut Transaction<'_, Postgres>,
    input: &UrlCaptureRows<'_>,
) -> AppResult<Uuid> {
    sqlx::query_scalar(INSERT_CAPTURE_ITEM)
        .bind(input.user_id)
        .bind(input.client_capture_id)
        .bind(ItemKind::Url.as_str())
        .fetch_one(&mut **transaction)
        .await
        .map_err(database_error)
}

async fn insert_capture_url(
    transaction: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    input: &UrlCaptureRows<'_>,
) -> AppResult<bool> {
    let result = sqlx::query(INSERT_CAPTURE_URL)
        .bind(item_id)
        .bind(input.user_id)
        .bind(input.original_url)
        .bind(input.normalized_url.canonical_url.as_deref())
        .bind(input.normalized_url.normalization_status.as_str())
        .bind(input.normalized_url.normalization_error.as_deref())
        .execute(&mut **transaction)
        .await
        .map_err(database_error)?;
    Ok(result.rows_affected() > 0)
}

async fn insert_capture_title(
    transaction: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    user_id: Uuid,
    title: Option<&str>,
) -> AppResult<()> {
    if let Some(title) = title {
        sqlx::query(INSERT_CAPTURE_TITLE)
            .bind(item_id)
            .bind(user_id)
            .bind(title)
            .execute(&mut **transaction)
            .await
            .map_err(database_error)?;
    }
    Ok(())
}

async fn insert_capture_tags(
    transaction: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    user_id: Uuid,
    tags: &[TagName],
) -> AppResult<()> {
    for tag in tags {
        let tag_id: Uuid = sqlx::query_scalar(UPSERT_TAG)
            .bind(user_id)
            .bind(tag.display_name())
            .fetch_one(&mut **transaction)
            .await
            .map_err(database_error)?;
        sqlx::query(INSERT_ITEM_TAG)
            .bind(item_id)
            .bind(tag_id)
            .bind(user_id)
            .execute(&mut **transaction)
            .await
            .map_err(database_error)?;
    }
    Ok(())
}
