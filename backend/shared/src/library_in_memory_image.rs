use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::UserContext;
use crate::domain::{ArchiveStatus, ImageUploadStatus, InboxStatus, ItemKind, WatchStatus};
use crate::error::{AppError, AppResult};
use crate::library::{
    CaptureImageUploadRequest, CaptureItemOutcome, ItemImageSummary, ItemTag, LibraryItemDetail,
    LibraryItemSummary,
};

use super::{tag_ops, validate_client_capture_id, validation_error, InMemoryLibraryService};

pub(super) async fn capture_image_upload(
    service: &InMemoryLibraryService,
    user: &UserContext,
    request: CaptureImageUploadRequest,
) -> AppResult<CaptureItemOutcome> {
    let request = ValidImageRequest::new(request)?;
    let client_capture_id = validate_client_capture_id(request.client_capture_id.clone())?;
    if let Some(item) = service.existing_capture(user, client_capture_id.as_deref())? {
        return Ok(existing_outcome(item));
    }
    let tags = tag_ops::capture_tags(&service.tags_by_user, &user.sub, &request.tags)?;
    let item = new_image_item(request, tags);
    service.store_capture(user, client_capture_id, item.clone());
    Ok(CaptureItemOutcome {
        item,
        created: true,
    })
}

pub(super) fn complete_image_upload(
    service: &InMemoryLibraryService,
    user: &UserContext,
    item_id: Uuid,
) -> AppResult<LibraryItemDetail> {
    let mut items = service.items_by_user.lock().unwrap();
    let user_items = items.get_mut(&user.sub).ok_or_else(|| not_found(item_id))?;
    let item = user_items
        .iter_mut()
        .find(|item| item.summary.id == item_id)
        .ok_or_else(|| not_found(item_id))?;
    let image = item
        .summary
        .image
        .as_mut()
        .ok_or_else(|| not_found(item_id))?;
    image.upload_status = ImageUploadStatus::Uploaded;
    item.summary.archive_status = ArchiveStatus::Succeeded;
    Ok(item.clone())
}

struct ValidImageRequest {
    content_type: String,
    title: Option<String>,
    original_filename: Option<String>,
    byte_size: Option<i64>,
    source_app: Option<String>,
    source_device: Option<String>,
    capture_method: String,
    tags: Vec<String>,
    client_capture_id: Option<String>,
}

impl ValidImageRequest {
    fn new(request: CaptureImageUploadRequest) -> AppResult<Self> {
        Ok(Self {
            content_type: validate_image_content_type(request.content_type)?,
            title: clean_optional(request.title),
            original_filename: clean_optional(request.original_filename),
            byte_size: validate_byte_size(request.byte_size)?,
            source_app: clean_optional(request.source_app),
            source_device: clean_optional(request.source_device),
            capture_method: capture_method(request.capture_method),
            tags: request.tags,
            client_capture_id: request.client_capture_id,
        })
    }
}

fn new_image_item(request: ValidImageRequest, tags: Vec<ItemTag>) -> LibraryItemDetail {
    let item_id = Uuid::new_v4();
    LibraryItemDetail {
        summary: LibraryItemSummary {
            id: item_id,
            item_kind: ItemKind::Image,
            url: None,
            text: None,
            image: Some(ItemImageSummary {
                s3_key: image_s3_key(item_id),
                content_type: request.content_type,
                original_filename: request.original_filename,
                byte_size: request.byte_size,
                upload_status: ImageUploadStatus::Pending,
                source_app: request.source_app,
                source_device: request.source_device,
                capture_method: request.capture_method,
            }),
            title: request.title,
            fetched_title: None,
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

fn capture_method(value: Option<String>) -> String {
    clean_optional(value).unwrap_or_else(|| "android_share".to_string())
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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
