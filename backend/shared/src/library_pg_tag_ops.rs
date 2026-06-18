use uuid::Uuid;

use crate::db::DbPool;
use crate::domain::TagName;
use crate::error::{AppError, AppResult};
use crate::library::{MergeTagsRequest, RenameTagRequest};
use crate::library_pg_capture_helpers::validation_error;

use super::database_error;
use super::library_pg_sql::{
    DELETE_SOURCE_ITEM_TAGS, DELETE_TAG_BY_ID, MERGE_ITEM_TAGS, TAG_BY_ID, TAG_RENAME_COLLISION,
    UPDATE_TAG_DISPLAY_NAME,
};

pub(super) async fn rename_tag(
    db: &DbPool,
    user_id: Uuid,
    tag_id: Uuid,
    request: RenameTagRequest,
) -> AppResult<()> {
    let tag = TagName::new(&request.display_name).map_err(validation_error)?;
    reject_rename_collision(db, user_id, tag_id, tag.normalized_name()).await?;
    let result = sqlx::query(UPDATE_TAG_DISPLAY_NAME)
        .bind(tag_id)
        .bind(user_id)
        .bind(tag.display_name())
        .execute(db)
        .await
        .map_err(database_error)?;
    if result.rows_affected() == 0 {
        return Err(tag_not_found(tag_id));
    }
    Ok(())
}

pub(super) async fn merge_tags(
    db: &DbPool,
    user_id: Uuid,
    source_tag_id: Uuid,
    request: MergeTagsRequest,
) -> AppResult<()> {
    if source_tag_id == request.target_tag_id {
        return Err(AppError::Validation(
            "cannot merge a tag into itself".to_string(),
        ));
    }
    let mut transaction = db.begin().await.map_err(database_error)?;
    ensure_tag_owned(&mut transaction, user_id, source_tag_id).await?;
    ensure_tag_owned(&mut transaction, user_id, request.target_tag_id).await?;
    sqlx::query(MERGE_ITEM_TAGS)
        .bind(source_tag_id)
        .bind(user_id)
        .bind(request.target_tag_id)
        .execute(&mut *transaction)
        .await
        .map_err(database_error)?;
    sqlx::query(DELETE_SOURCE_ITEM_TAGS)
        .bind(source_tag_id)
        .bind(user_id)
        .execute(&mut *transaction)
        .await
        .map_err(database_error)?;
    sqlx::query(DELETE_TAG_BY_ID)
        .bind(source_tag_id)
        .bind(user_id)
        .execute(&mut *transaction)
        .await
        .map_err(database_error)?;
    transaction.commit().await.map_err(database_error)?;
    Ok(())
}

pub(super) fn tag_not_found(tag_id: Uuid) -> AppError {
    AppError::NotFound(format!("tag {tag_id}"))
}

async fn reject_rename_collision(
    db: &DbPool,
    user_id: Uuid,
    tag_id: Uuid,
    normalized_name: &str,
) -> AppResult<()> {
    let collision: Option<Uuid> = sqlx::query_scalar(TAG_RENAME_COLLISION)
        .bind(user_id)
        .bind(normalized_name)
        .bind(tag_id)
        .fetch_optional(db)
        .await
        .map_err(database_error)?;
    if collision.is_some() {
        return Err(AppError::Validation("tag name already exists".to_string()));
    }
    Ok(())
}

async fn ensure_tag_owned(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    tag_id: Uuid,
) -> AppResult<()> {
    sqlx::query_scalar::<_, Uuid>(TAG_BY_ID)
        .bind(tag_id)
        .bind(user_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(database_error)?
        .ok_or_else(|| tag_not_found(tag_id))?;
    Ok(())
}
