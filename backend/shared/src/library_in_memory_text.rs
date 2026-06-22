use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::UserContext;
use crate::domain::{ArchiveStatus, InboxStatus, ItemKind, TextSnippetBody, WatchStatus};
use crate::error::AppResult;
use crate::library::{
    CaptureItemOutcome, CaptureTextRequest, ItemTag, ItemTextSummary, LibraryItemDetail,
    LibraryItemSummary,
};

use super::{tag_ops, validate_client_capture_id, validation_error, InMemoryLibraryService};

pub(super) async fn capture_text(
    service: &InMemoryLibraryService,
    user: &UserContext,
    request: CaptureTextRequest,
) -> AppResult<CaptureItemOutcome> {
    let plain_text = TextSnippetBody::new(request.plain_text.clone())
        .map_err(validation_error)?
        .into_string();
    let client_capture_id = validate_client_capture_id(request.client_capture_id.clone())?;
    if let Some(item) = service.existing_capture(user, client_capture_id.as_deref())? {
        return Ok(existing_outcome(item));
    }

    let content_hash = content_hash(&plain_text, request.html.as_deref());
    if let Some(item) = existing_text_capture(service, user, &content_hash) {
        return Ok(existing_outcome(item));
    }

    let item = new_text_item(
        plain_text,
        content_hash,
        &request,
        tag_ops::capture_tags(&service.tags_by_user, &user.sub, &request.tags)?,
    );
    service.store_capture(user, client_capture_id, item.clone());
    Ok(CaptureItemOutcome {
        item,
        created: true,
    })
}

fn existing_text_capture(
    service: &InMemoryLibraryService,
    user: &UserContext,
    content_hash: &str,
) -> Option<LibraryItemDetail> {
    service
        .items_by_user
        .lock()
        .unwrap()
        .get(&user.sub)
        .and_then(|items| text_match(items, content_hash))
}

fn text_match(items: &[LibraryItemDetail], content_hash: &str) -> Option<LibraryItemDetail> {
    items
        .iter()
        .find(|item| {
            item.summary
                .text
                .as_ref()
                .map(|text| text.content_hash.as_str())
                == Some(content_hash)
        })
        .cloned()
}

fn new_text_item(
    plain_text: String,
    content_hash: String,
    request: &CaptureTextRequest,
    tags: Vec<ItemTag>,
) -> LibraryItemDetail {
    LibraryItemDetail {
        summary: LibraryItemSummary {
            id: Uuid::new_v4(),
            item_kind: ItemKind::TextSnippet,
            url: None,
            text: Some(ItemTextSummary::new(
                plain_text,
                clean_optional(request.html.clone()),
                content_hash,
                clean_optional(request.source_app.clone()),
                clean_optional(request.source_device.clone()),
                capture_method(request.capture_method.clone()),
            )),
            image: None,
            title: clean_optional(request.title.clone()),
            fetched_title: None,
            thumbnail_s3_key: None,
            author: None,
            platform: None,
            duration_seconds: None,
            archive_status: ArchiveStatus::NotApplicable,
            watch_status: WatchStatus::Unwatched,
            inbox_status: InboxStatus::Unsorted,
            tags,
            created_at: OffsetDateTime::now_utc(),
        },
        notes: String::new(),
    }
}

fn existing_outcome(item: LibraryItemDetail) -> CaptureItemOutcome {
    CaptureItemOutcome {
        item,
        created: false,
    }
}

fn content_hash(plain_text: &str, html: Option<&str>) -> String {
    let mut sha = Sha256::new();
    sha.update(plain_text.as_bytes());
    sha.update([0]);
    if let Some(html) = html {
        sha.update(html.as_bytes());
    }
    STANDARD.encode(sha.finalize())
}

fn clean_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn capture_method(value: Option<String>) -> String {
    clean_optional(value).unwrap_or_else(|| "desktop_clipboard".to_string())
}
