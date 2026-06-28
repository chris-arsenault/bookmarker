use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use shared::domain::{ArchiveStatus, ImageUploadStatus, InboxStatus, WatchStatus};
use shared::library::{
    CaptureImageUploadRequest, CaptureItemOutcome, CaptureItemRequest, CaptureTextRequest,
    LibraryItemDetail, LibraryItemSummary, LibraryUpdates, ListItemUpdatesQuery, ListItemsQuery,
    UpdateItemRequest,
};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::{require_user, ApiError, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/items", get(list_items).post(capture_item))
        .route("/items/updates", get(list_item_updates))
        .route("/items/text", post(capture_text))
        .route("/items/images/uploads", post(capture_image_upload))
        .route("/items/{item_id}/image", get(get_item_image))
        .route(
            "/items/{item_id}/image-upload/complete",
            post(complete_image_upload),
        )
        .route("/items/{item_id}/thumbnail", get(get_item_thumbnail))
        .route(
            "/items/{item_id}",
            get(get_item).patch(update_item).delete(delete_item),
        )
}

#[derive(Debug, Serialize)]
struct CaptureImageUploadOutcome {
    item: LibraryItemDetail,
    created: bool,
    upload: crate::image_access::ImageUploadTarget,
}

#[derive(Debug, Serialize)]
struct ImageAccessOutcome {
    view_url: String,
    download_url: String,
    content_type: String,
    download_name: String,
    expires_in_seconds: u64,
}

async fn capture_image_upload(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<CaptureImageUploadRequest>,
) -> Result<(StatusCode, Json<CaptureImageUploadOutcome>), ApiError> {
    let user = require_user(&state, &headers).await?;
    let outcome = state.library.capture_image_upload(&user, request).await?;
    let image = outcome
        .item
        .summary
        .image
        .as_ref()
        .ok_or_else(|| validation_error("image upload did not create image metadata"))?;
    let upload = state
        .image_store
        .upload_target(&image.s3_key, &image.content_type)
        .await?;
    let status = if outcome.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((
        status,
        Json(CaptureImageUploadOutcome {
            item: outcome.item,
            created: outcome.created,
            upload,
        }),
    ))
}

async fn complete_image_upload(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(item_id): Path<Uuid>,
) -> Result<Json<LibraryItemDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    Ok(Json(
        state.library.complete_image_upload(&user, item_id).await?,
    ))
}

async fn capture_text(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<CaptureTextRequest>,
) -> Result<(StatusCode, Json<CaptureItemOutcome>), ApiError> {
    let user = require_user(&state, &headers).await?;
    let outcome = state.library.capture_text(&user, request).await?;
    let status = if outcome.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(outcome)))
}

async fn capture_item(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<CaptureItemRequest>,
) -> Result<(StatusCode, Json<CaptureItemOutcome>), ApiError> {
    let user = require_user(&state, &headers).await?;
    let outcome = state.library.capture_item(&user, request).await?;
    let status = if outcome.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    dispatch_processing(&state, outcome.item.summary.id).await;
    Ok((status, Json(outcome)))
}

async fn dispatch_processing(state: &ApiState, item_id: Uuid) {
    if let Err(err) = state.processing_dispatcher.dispatch_item(item_id).await {
        tracing::warn!(
            item_id = %item_id,
            error = %err,
            "failed to dispatch processing"
        );
    }
}

async fn list_items(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Query(params): Query<ListItemsParams>,
) -> Result<Json<Vec<LibraryItemSummary>>, ApiError> {
    let user = require_user(&state, &headers).await?;
    let query = params.into_query()?;
    Ok(Json(state.library.list_items(&user, &query).await?))
}

async fn list_item_updates(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Query(params): Query<ListItemUpdatesParams>,
) -> Result<Json<LibraryUpdates>, ApiError> {
    let user = require_user(&state, &headers).await?;
    let query = params.into_query()?;
    Ok(Json(state.library.list_item_updates(&user, &query).await?))
}

async fn get_item(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(item_id): Path<Uuid>,
) -> Result<Json<LibraryItemDetail>, ApiError> {
    let user = require_user(&state, &headers).await?;
    Ok(Json(state.library.get_item(&user, item_id).await?))
}

async fn get_item_thumbnail(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(item_id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let user = require_user(&state, &headers).await?;
    let item = state.library.get_item(&user, item_id).await?;
    let key = item.summary.thumbnail_s3_key.ok_or_else(|| {
        shared::error::AppError::NotFound(format!("thumbnail for item {item_id}"))
    })?;
    let object = state.thumbnail_reader.read_thumbnail(&key).await?;
    Ok(binary_response(object.content_type, object.bytes))
}

async fn get_item_image(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(item_id): Path<Uuid>,
) -> Result<Json<ImageAccessOutcome>, ApiError> {
    let user = require_user(&state, &headers).await?;
    let item = state.library.get_item(&user, item_id).await?;
    let image =
        item.summary.image.as_ref().ok_or_else(|| {
            shared::error::AppError::NotFound(format!("image for item {item_id}"))
        })?;
    if image.upload_status != ImageUploadStatus::Uploaded {
        return Err(shared::error::AppError::NotFound(format!("image for item {item_id}")).into());
    }
    let download_name = image_download_name(&item.summary);
    let access = state
        .image_store
        .access_target(&image.s3_key, &image.content_type, &download_name)
        .await?;
    Ok(Json(ImageAccessOutcome {
        view_url: access.view_url,
        download_url: access.download_url,
        content_type: image.content_type.clone(),
        download_name,
        expires_in_seconds: access.expires_in_seconds,
    }))
}

fn binary_response(content_type: String, bytes: Vec<u8>) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type),
            (
                header::CACHE_CONTROL,
                "private, max-age=31536000, immutable".to_string(),
            ),
        ],
        bytes,
    )
        .into_response()
}

fn image_download_name(summary: &LibraryItemSummary) -> String {
    summary
        .image
        .as_ref()
        .and_then(|image| image.original_filename.clone())
        .unwrap_or_else(|| format!("{}.image", summary.id))
}

async fn update_item(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(item_id): Path<Uuid>,
    Json(request): Json<UpdateItemRequest>,
) -> Result<Json<LibraryItemDetail>, ApiError> {
    if empty_update_request(&request) {
        return Err(validation_error("item update must include at least one field").into());
    }
    let user = require_user(&state, &headers).await?;
    Ok(Json(
        state.library.update_item(&user, item_id, request).await?,
    ))
}

async fn delete_item(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(item_id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let user = require_user(&state, &headers).await?;
    state.library.delete_item(&user, item_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Default, Deserialize)]
struct ListItemsParams {
    platform: Option<String>,
    tag: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    archive_status: Option<String>,
    watch_status: Option<String>,
    inbox_status: Option<String>,
    q: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ListItemUpdatesParams {
    since: Option<String>,
    limit: Option<i64>,
    platform: Option<String>,
    tag: Option<String>,
    created_from: Option<String>,
    created_to: Option<String>,
    archive_status: Option<String>,
    watch_status: Option<String>,
    inbox_status: Option<String>,
    q: Option<String>,
}

impl ListItemsParams {
    fn into_query(self) -> Result<ListItemsQuery, ApiError> {
        Ok(ListItemsQuery {
            platform: clean_param(self.platform),
            tag: clean_param(self.tag),
            created_from: parse_datetime_param("created_from", self.created_from)?,
            created_to: parse_datetime_param("created_to", self.created_to)?,
            archive_status: parse_archive_status(self.archive_status)?,
            watch_status: parse_watch_status(self.watch_status)?,
            inbox_status: parse_inbox_status(self.inbox_status)?,
            q: clean_param(self.q),
        })
    }
}

impl ListItemUpdatesParams {
    fn into_query(self) -> Result<ListItemUpdatesQuery, ApiError> {
        Ok(ListItemUpdatesQuery {
            since: parse_datetime_param("since", self.since)?,
            limit: update_limit(self.limit)?,
            filters: ListItemsParams {
                platform: self.platform,
                tag: self.tag,
                created_from: self.created_from,
                created_to: self.created_to,
                archive_status: self.archive_status,
                watch_status: self.watch_status,
                inbox_status: self.inbox_status,
                q: self.q,
            }
            .into_query()?,
        })
    }
}

fn update_limit(value: Option<i64>) -> Result<i64, ApiError> {
    const DEFAULT_LIMIT: i64 = 100;
    const MAX_LIMIT: i64 = 250;
    match value.unwrap_or(DEFAULT_LIMIT) {
        limit if limit <= 0 => Err(validation_error("limit must be positive").into()),
        limit => Ok(limit.min(MAX_LIMIT)),
    }
}

fn parse_archive_status(value: Option<String>) -> Result<Option<ArchiveStatus>, ApiError> {
    clean_param(value)
        .map(|value| ArchiveStatus::try_from(value.as_str()).map_err(validation_error))
        .transpose()
        .map_err(Into::into)
}

fn parse_watch_status(value: Option<String>) -> Result<Option<WatchStatus>, ApiError> {
    clean_param(value)
        .map(|value| WatchStatus::try_from(value.as_str()).map_err(validation_error))
        .transpose()
        .map_err(Into::into)
}

fn parse_inbox_status(value: Option<String>) -> Result<Option<InboxStatus>, ApiError> {
    clean_param(value)
        .map(|value| InboxStatus::try_from(value.as_str()).map_err(validation_error))
        .transpose()
        .map_err(Into::into)
}

fn parse_datetime_param(
    name: &'static str,
    value: Option<String>,
) -> Result<Option<OffsetDateTime>, ApiError> {
    clean_param(value)
        .map(|value| {
            OffsetDateTime::parse(&value, &Rfc3339)
                .map_err(|_| shared::error::AppError::Validation(format!("{name} must be RFC3339")))
        })
        .transpose()
        .map_err(Into::into)
}

fn empty_update_request(request: &UpdateItemRequest) -> bool {
    request.title.is_none()
        && request.watch_status.is_none()
        && request.inbox_status.is_none()
        && request.notes.is_none()
        && request.tags.is_none()
}

fn clean_param(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validation_error(err: impl ToString) -> shared::error::AppError {
    shared::error::AppError::Validation(err.to_string())
}
