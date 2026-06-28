use crate::http::prelude::*;
use serde::Serialize;
use shared::domain::ImageUploadStatus;
use shared::library::{
    CaptureImageUploadRequest, CaptureItemOutcome, CaptureItemRequest, CaptureTextRequest,
    LibraryItemDetail, LibraryItemSummary, UpdateItemRequest,
};
use uuid::Uuid;

use crate::item_operation_details::{
    capture_image_upload_operation, capture_text_operation, capture_url_operation,
    complete_image_upload_operation, dispatch_item_operation, item_operation,
    list_item_updates_operation, list_items_operation, update_item_operation,
};
use crate::item_query::{ListItemUpdatesParams, ListItemsParams};
use crate::{observe_api_operation, require_user, ApiResponse, ApiResult, ApiState};

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

pub async fn dispatch(
    route: &Route<'_>,
    request: &Request,
    state: &ApiState,
) -> ApiResult<Option<ApiResponse>> {
    if let Some(response) = collection_route(route, request, state).await? {
        return Ok(Some(response));
    }
    if let Some(response) = image_upload_route(route, request, state).await? {
        return Ok(Some(response));
    }
    if let Some(response) = item_asset_route(route, request, state).await? {
        return Ok(Some(response));
    }
    item_route(route, request, state).await
}

async fn collection_route(
    route: &Route<'_>,
    request: &Request,
    state: &ApiState,
) -> ApiResult<Option<ApiResponse>> {
    if route.is_match(Method::GET, "/items")? {
        return list_items(state, request).await.map(Some);
    }
    if route.is_match(Method::POST, "/items")? {
        return capture_item(state, request).await.map(Some);
    }
    if route.is_match(Method::GET, "/items/updates")? {
        return list_item_updates(state, request).await.map(Some);
    }
    if route.is_match(Method::POST, "/items/text")? {
        return capture_text(state, request).await.map(Some);
    }
    Ok(None)
}

async fn image_upload_route(
    route: &Route<'_>,
    request: &Request,
    state: &ApiState,
) -> ApiResult<Option<ApiResponse>> {
    if route.is_match(Method::POST, "/items/images/uploads")? {
        return capture_image_upload(state, request).await.map(Some);
    }
    if let Some(params) = route.matches(Method::POST, "/items/{item_id}/image-upload/complete")? {
        return complete_image_upload(state, request, params.parse("item_id")?)
            .await
            .map(Some);
    }
    Ok(None)
}

async fn item_asset_route(
    route: &Route<'_>,
    request: &Request,
    state: &ApiState,
) -> ApiResult<Option<ApiResponse>> {
    if let Some(params) = route.matches(Method::GET, "/items/{item_id}/image")? {
        return get_item_image(state, request, params.parse("item_id")?)
            .await
            .map(Some);
    }
    if let Some(params) = route.matches(Method::GET, "/items/{item_id}/thumbnail")? {
        return get_item_thumbnail(state, request, params.parse("item_id")?)
            .await
            .map(Some);
    }
    Ok(None)
}

async fn item_route(
    route: &Route<'_>,
    request: &Request,
    state: &ApiState,
) -> ApiResult<Option<ApiResponse>> {
    if let Some(params) = route.matches(Method::GET, "/items/{item_id}")? {
        return get_item(state, request, params.parse("item_id")?)
            .await
            .map(Some);
    }
    if let Some(params) = route.matches(Method::PATCH, "/items/{item_id}")? {
        return update_item(state, request, params.parse("item_id")?)
            .await
            .map(Some);
    }
    if let Some(params) = route.matches(Method::DELETE, "/items/{item_id}")? {
        return delete_item(state, request, params.parse("item_id")?)
            .await
            .map(Some);
    }
    Ok(None)
}

async fn capture_image_upload(state: &ApiState, request: &Request) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    let capture = json_body::<CaptureImageUploadRequest>(request)?;
    observe_api_operation(capture_image_upload_operation(&user, &capture), async {
        let outcome = state.library.capture_image_upload(&user, capture).await?;
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
        json_response(
            created_status(outcome.created),
            &CaptureImageUploadOutcome {
                item: outcome.item,
                created: outcome.created,
                upload,
            },
        )
        .map_err(Into::into)
    })
    .await
}

async fn complete_image_upload(
    state: &ApiState,
    request: &Request,
    item_id: Uuid,
) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    let existing = state.library.get_item(&user, item_id).await?;
    observe_api_operation(
        complete_image_upload_operation(&user, item_id, existing.summary.image.as_ref()),
        async {
            json_response(
                StatusCode::OK,
                &state.library.complete_image_upload(&user, item_id).await?,
            )
            .map_err(Into::into)
        },
    )
    .await
}

async fn capture_text(state: &ApiState, request: &Request) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    let capture = json_body::<CaptureTextRequest>(request)?;
    observe_api_operation(capture_text_operation(&user, &capture), async {
        let outcome = state.library.capture_text(&user, capture).await?;
        capture_outcome_response(outcome)
    })
    .await
}

async fn capture_item(state: &ApiState, request: &Request) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    let capture = json_body::<CaptureItemRequest>(request)?;
    observe_api_operation(capture_url_operation(&user, &capture), async {
        let outcome = state.library.capture_item(&user, capture).await?;
        dispatch_processing(state, outcome.item.summary.id).await;
        capture_outcome_response(outcome)
    })
    .await
}

async fn dispatch_processing(state: &ApiState, item_id: Uuid) {
    let result = observe_api_operation(dispatch_item_operation(item_id), async {
        state.processing_dispatcher.dispatch_item(item_id).await
    })
    .await;
    if let Err(err) = result {
        tracing::warn!(
            item_id = %item_id,
            error = %err,
            "failed to dispatch processing"
        );
    }
}

async fn list_items(state: &ApiState, request: &Request) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    let query = query_params::<ListItemsParams>(request)?.into_query()?;
    observe_api_operation(list_items_operation(&user, &query), async {
        json_response(
            StatusCode::OK,
            &state.library.list_items(&user, &query).await?,
        )
        .map_err(Into::into)
    })
    .await
}

async fn list_item_updates(state: &ApiState, request: &Request) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    let query = query_params::<ListItemUpdatesParams>(request)?.into_query()?;
    observe_api_operation(list_item_updates_operation(&user, &query), async {
        json_response(
            StatusCode::OK,
            &state.library.list_item_updates(&user, &query).await?,
        )
        .map_err(Into::into)
    })
    .await
}

async fn get_item(state: &ApiState, request: &Request, item_id: Uuid) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    observe_api_operation(item_operation("api.items.get", &user, item_id), async {
        json_response(
            StatusCode::OK,
            &state.library.get_item(&user, item_id).await?,
        )
        .map_err(Into::into)
    })
    .await
}

async fn get_item_thumbnail(
    state: &ApiState,
    request: &Request,
    item_id: Uuid,
) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    let operation = item_operation("api.items.get_thumbnail", &user, item_id)
        .with_detail("asset.kind", "thumbnail");
    observe_api_operation(operation, async {
        let item = state.library.get_item(&user, item_id).await?;
        let key = item.summary.thumbnail_s3_key.ok_or_else(|| {
            shared::error::AppError::NotFound(format!("thumbnail for item {item_id}"))
        })?;
        let object = state.thumbnail_reader.read_thumbnail(&key).await?;
        binary_response(StatusCode::OK, object.content_type, object.bytes)
            .map(private_immutable_cache)
            .map_err(Into::into)
    })
    .await
}

async fn get_item_image(
    state: &ApiState,
    request: &Request,
    item_id: Uuid,
) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    let operation =
        item_operation("api.items.get_image", &user, item_id).with_detail("asset.kind", "image");
    observe_api_operation(operation, async {
        let item = state.library.get_item(&user, item_id).await?;
        let image = item.summary.image.as_ref().ok_or_else(|| {
            shared::error::AppError::NotFound(format!("image for item {item_id}"))
        })?;
        if image.upload_status != ImageUploadStatus::Uploaded {
            return Err(
                shared::error::AppError::NotFound(format!("image for item {item_id}")).into(),
            );
        }
        let download_name = image_download_name(&item.summary);
        let access = state
            .image_store
            .access_target(&image.s3_key, &image.content_type, &download_name)
            .await?;
        json_response(
            StatusCode::OK,
            &ImageAccessOutcome {
                view_url: access.view_url,
                download_url: access.download_url,
                content_type: image.content_type.clone(),
                download_name,
                expires_in_seconds: access.expires_in_seconds,
            },
        )
        .map_err(Into::into)
    })
    .await
}

fn image_download_name(summary: &LibraryItemSummary) -> String {
    summary
        .image
        .as_ref()
        .and_then(|image| image.original_filename.clone())
        .unwrap_or_else(|| format!("{}.image", summary.id))
}

async fn update_item(state: &ApiState, request: &Request, item_id: Uuid) -> ApiResult<ApiResponse> {
    let update = json_body::<UpdateItemRequest>(request)?;
    if empty_update_request(&update) {
        return Err(validation_error("item update must include at least one field").into());
    }
    let user = require_user(state, request.headers()).await?;
    observe_api_operation(update_item_operation(&user, item_id, &update), async {
        json_response(
            StatusCode::OK,
            &state.library.update_item(&user, item_id, update).await?,
        )
        .map_err(Into::into)
    })
    .await
}

async fn delete_item(state: &ApiState, request: &Request, item_id: Uuid) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    observe_api_operation(item_operation("api.items.delete", &user, item_id), async {
        state.library.delete_item(&user, item_id).await?;
        Ok(no_content_response())
    })
    .await
}

fn capture_outcome_response(outcome: CaptureItemOutcome) -> ApiResult<ApiResponse> {
    json_response(created_status(outcome.created), &outcome).map_err(Into::into)
}

fn created_status(created: bool) -> StatusCode {
    if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    }
}

fn empty_update_request(request: &UpdateItemRequest) -> bool {
    request.title.is_none()
        && request.watch_status.is_none()
        && request.inbox_status.is_none()
        && request.notes.is_none()
        && request.tags.is_none()
}

fn validation_error(err: impl ToString) -> shared::error::AppError {
    shared::error::AppError::Validation(err.to_string())
}
