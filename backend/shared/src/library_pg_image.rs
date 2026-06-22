use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::UserContext;
use crate::domain::{ImageUploadStatus, ItemKind, TagName};
use crate::error::{AppError, AppResult};
use crate::library::{
    CaptureImageUploadRequest, CaptureItemOutcome, LibraryItemDetail, LibraryService,
};
use crate::library_pg_capture_helpers::{
    validate_client_capture_id, validate_tags, validation_error,
};

use super::{
    database_error, PgLibraryService, INSERT_CAPTURE_ITEM, INSERT_IMAGE_CAPTURE, INSERT_ITEM_TAG,
    UPDATE_IMAGE_UPLOAD_STATUS, UPSERT_TAG,
};

pub(super) async fn capture_image_upload(
    service: &PgLibraryService,
    user: &UserContext,
    request: CaptureImageUploadRequest,
) -> AppResult<CaptureItemOutcome> {
    let request = ValidImageUploadRequest::new(request)?;
    let client_capture_id = validate_client_capture_id(request.client_capture_id.clone())?;
    let user_id = service.upsert_user(user).await?;
    if let Some(item) = service
        .existing_capture(user_id, client_capture_id.as_deref())
        .await?
    {
        return Ok(existing_outcome(item));
    }
    insert_image_upload(service, user, user_id, client_capture_id, request).await
}

pub(super) async fn complete_image_upload(
    service: &PgLibraryService,
    user: &UserContext,
    item_id: Uuid,
) -> AppResult<LibraryItemDetail> {
    let user_id = service.required_user_id(user, item_id).await?;
    let result = sqlx::query(UPDATE_IMAGE_UPLOAD_STATUS)
        .bind(item_id)
        .bind(user_id)
        .bind(ImageUploadStatus::Uploaded.as_str())
        .execute(&service.db)
        .await
        .map_err(database_error)?;
    if result.rows_affected() == 0 {
        return Err(not_found(item_id));
    }
    service.get_item(user, item_id).await
}

struct ValidImageUploadRequest {
    content_type: String,
    title: Option<String>,
    original_filename: Option<String>,
    byte_size: Option<i64>,
    source_app: Option<String>,
    source_device: Option<String>,
    capture_method: String,
    tags: Vec<TagName>,
    client_capture_id: Option<String>,
}

impl ValidImageUploadRequest {
    fn new(request: CaptureImageUploadRequest) -> AppResult<Self> {
        Ok(Self {
            content_type: validate_image_content_type(request.content_type)?,
            title: clean_optional(request.title),
            original_filename: clean_optional(request.original_filename),
            byte_size: validate_byte_size(request.byte_size)?,
            source_app: clean_optional(request.source_app),
            source_device: clean_optional(request.source_device),
            capture_method: capture_method(request.capture_method),
            tags: validate_tags(&request.tags)?,
            client_capture_id: request.client_capture_id,
        })
    }
}

async fn insert_image_upload(
    service: &PgLibraryService,
    user: &UserContext,
    user_id: Uuid,
    client_capture_id: Option<String>,
    request: ValidImageUploadRequest,
) -> AppResult<CaptureItemOutcome> {
    let mut transaction = service.db.begin().await.map_err(database_error)?;
    let item_id = insert_image_item(
        &mut transaction,
        user_id,
        client_capture_id.as_deref(),
        request.title.as_deref(),
    )
    .await?;
    insert_image_payload(&mut transaction, item_id, user_id, &request).await?;
    attach_tags(&mut transaction, item_id, user_id, &request.tags).await?;
    transaction.commit().await.map_err(database_error)?;
    Ok(CaptureItemOutcome {
        item: service.get_item(user, item_id).await?,
        created: true,
    })
}

async fn insert_image_item(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    client_capture_id: Option<&str>,
    title: Option<&str>,
) -> AppResult<Uuid> {
    sqlx::query_scalar(INSERT_CAPTURE_ITEM)
        .bind(user_id)
        .bind(client_capture_id)
        .bind(ItemKind::Image.as_str())
        .bind(title)
        .fetch_one(&mut **transaction)
        .await
        .map_err(database_error)
}

async fn insert_image_payload(
    transaction: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    user_id: Uuid,
    request: &ValidImageUploadRequest,
) -> AppResult<()> {
    sqlx::query(INSERT_IMAGE_CAPTURE)
        .bind(item_id)
        .bind(user_id)
        .bind(image_s3_key(item_id))
        .bind(&request.content_type)
        .bind(request.original_filename.as_deref())
        .bind(request.byte_size)
        .bind(request.source_app.as_deref())
        .bind(request.source_device.as_deref())
        .bind(&request.capture_method)
        .execute(&mut **transaction)
        .await
        .map_err(database_error)?;
    Ok(())
}

async fn attach_tags(
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

fn image_s3_key(item_id: Uuid) -> String {
    format!("images/{item_id}/original")
}

fn validate_image_content_type(value: String) -> AppResult<String> {
    let value = value.trim().to_ascii_lowercase();
    if value.starts_with("image/") && value.len() > "image/".len() {
        return Ok(value);
    }
    Err(validation_error("content_type must be an image media type"))
}

fn validate_byte_size(value: Option<i64>) -> AppResult<Option<i64>> {
    match value {
        Some(value) if value <= 0 => Err(validation_error("byte_size must be positive")),
        value => Ok(value),
    }
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn capture_method(value: Option<String>) -> String {
    clean_optional(value).unwrap_or_else(|| "android_share".to_string())
}

fn existing_outcome(item: LibraryItemDetail) -> CaptureItemOutcome {
    CaptureItemOutcome {
        item,
        created: false,
    }
}

fn not_found(item_id: Uuid) -> AppError {
    AppError::NotFound(format!("image item {item_id}"))
}
