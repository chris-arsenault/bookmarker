use ahara_lambda_telemetry::{Operation, OperationKind};
use shared::auth::UserContext;
use shared::library::{
    CaptureImageUploadRequest, CaptureItemRequest, CaptureTextRequest, ItemImageSummary,
    LibraryItemSummary, ListItemUpdatesQuery, ListItemsQuery, UpdateItemRequest,
};
use url::Url;
use uuid::Uuid;

use crate::{api_operation, short_uuid_ref, user_api_operation};

pub(crate) fn capture_image_upload_operation(
    user: &UserContext,
    request: &CaptureImageUploadRequest,
) -> Operation {
    capture_source_details(
        user_api_operation("api.items.capture_image_upload", user)
            .with_detail("item.kind", "image")
            .with_optional_detail("item.title", title_detail(request.title.as_ref()))
            .with_detail("image.content_type", request.content_type.clone())
            .with_optional_detail("image.byte_size", request.byte_size)
            .with_optional_detail(
                "image.original_filename",
                request
                    .original_filename
                    .as_ref()
                    .and_then(|value| title_detail(Some(value))),
            )
            .with_detail("capture.title.present", request.title.is_some())
            .with_detail("capture.tag_count", request.tags.len() as i64)
            .with_detail(
                "client_capture_id.present",
                request.client_capture_id.is_some(),
            ),
        request.source_app.as_ref(),
        request.source_device.as_ref(),
        request.capture_method.as_ref(),
    )
}

pub(crate) fn complete_image_upload_operation(
    user: &UserContext,
    summary: &LibraryItemSummary,
) -> Operation {
    let operation = item_summary_details(
        user_api_operation("api.items.complete_image_upload", user),
        summary,
    );
    match summary.image.as_ref() {
        Some(image) => image_details(operation, image),
        None => operation,
    }
}

pub(crate) fn capture_text_operation(
    user: &UserContext,
    request: &CaptureTextRequest,
) -> Operation {
    capture_source_details(
        user_api_operation("api.items.capture_text", user)
            .with_detail("item.kind", "text_snippet")
            .with_optional_detail("item.title", title_detail(request.title.as_ref()))
            .with_detail(
                "text.length_chars",
                request.plain_text.chars().count() as i64,
            )
            .with_detail("text.html.present", request.html.is_some())
            .with_detail("capture.title.present", request.title.is_some())
            .with_detail("capture.tag_count", request.tags.len() as i64)
            .with_detail(
                "client_capture_id.present",
                request.client_capture_id.is_some(),
            ),
        request.source_app.as_ref(),
        request.source_device.as_ref(),
        request.capture_method.as_ref(),
    )
}

pub(crate) fn capture_url_operation(user: &UserContext, request: &CaptureItemRequest) -> Operation {
    user_api_operation("api.items.capture_url", user)
        .with_detail("item.kind", "url")
        .with_optional_detail("item.title", title_detail(request.title.as_ref()))
        .with_optional_detail("url.host", url_host(&request.url))
        .with_detail("capture.title.present", request.title.is_some())
        .with_detail("capture.tag_count", request.tags.len() as i64)
        .with_detail(
            "client_capture_id.present",
            request.client_capture_id.is_some(),
        )
}

pub(crate) fn dispatch_item_operation(item_id: Uuid) -> Operation {
    api_operation("api.processing.dispatch_item")
        .with_kind(OperationKind::Background)
        .with_detail("item.ref", short_uuid_ref(item_id))
}

pub(crate) fn list_items_operation(user: &UserContext, query: &ListItemsQuery) -> Operation {
    list_filter_details(user_api_operation("api.items.list", user), query)
}

pub(crate) fn list_item_updates_operation(
    user: &UserContext,
    query: &ListItemUpdatesQuery,
) -> Operation {
    list_filter_details(
        user_api_operation("api.items.list_updates", user)
            .with_kind(OperationKind::Polling)
            .with_detail("poll.since.present", query.since.is_some())
            .with_detail("poll.limit", query.limit),
        &query.filters,
    )
}

pub(crate) fn update_item_operation(
    user: &UserContext,
    item_id: Uuid,
    request: &UpdateItemRequest,
) -> Operation {
    item_operation("api.items.update", user, item_id)
        .with_detail("update.title.present", request.title.is_some())
        .with_detail(
            "update.watch_status.present",
            request.watch_status.is_some(),
        )
        .with_detail(
            "update.inbox_status.present",
            request.inbox_status.is_some(),
        )
        .with_detail("update.notes.present", request.notes.is_some())
        .with_optional_detail(
            "update.tag_count",
            request.tags.as_ref().map(|tags| tags.len() as i64),
        )
}

pub(crate) fn item_operation(name: &'static str, user: &UserContext, item_id: Uuid) -> Operation {
    user_api_operation(name, user).with_detail("item.ref", short_uuid_ref(item_id))
}

fn image_details(operation: Operation, image: &ItemImageSummary) -> Operation {
    capture_source_details(
        operation
            .with_detail("image.content_type", image.content_type.clone())
            .with_optional_detail("image.byte_size", image.byte_size)
            .with_detail("image.upload_status", image.upload_status.as_str())
            .with_optional_detail(
                "image.original_filename",
                image
                    .original_filename
                    .as_ref()
                    .and_then(|value| title_detail(Some(value))),
            ),
        image.source_app.as_ref(),
        image.source_device.as_ref(),
        Some(&image.capture_method),
    )
}

fn item_summary_details(operation: Operation, summary: &LibraryItemSummary) -> Operation {
    operation
        .with_detail("item.ref", short_uuid_ref(summary.id))
        .with_detail("item.kind", summary.item_kind.as_str())
        .with_optional_detail("item.title", title_detail(summary.title.as_ref()))
        .with_optional_detail(
            "metadata.fetched_title",
            title_detail(summary.fetched_title.as_ref()),
        )
        .with_optional_detail(
            "url.host",
            summary.url.as_ref().and_then(|url| url_host(&url.copy_url)),
        )
        .with_detail("tag.count", summary.tags.len() as i64)
}

fn title_detail(value: Option<&String>) -> Option<String> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    Some(value.chars().take(120).collect())
}

fn url_host(value: &str) -> Option<String> {
    Url::parse(value)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
}

fn capture_source_details(
    operation: Operation,
    source_app: Option<&String>,
    source_device: Option<&String>,
    capture_method: Option<&String>,
) -> Operation {
    operation
        .with_optional_detail("client.source_app", source_app.cloned())
        .with_detail("client.source_device.present", source_device.is_some())
        .with_optional_detail("capture.method", capture_method.cloned())
}

fn list_filter_details(operation: Operation, query: &ListItemsQuery) -> Operation {
    operation
        .with_detail("filter.platform.present", query.platform.is_some())
        .with_detail("filter.tag.present", query.tag.is_some())
        .with_detail("filter.created_from.present", query.created_from.is_some())
        .with_detail("filter.created_to.present", query.created_to.is_some())
        .with_detail(
            "filter.archive_status.present",
            query.archive_status.is_some(),
        )
        .with_detail("filter.watch_status.present", query.watch_status.is_some())
        .with_detail("filter.inbox_status.present", query.inbox_status.is_some())
        .with_detail("filter.q.present", query.q.is_some())
}
