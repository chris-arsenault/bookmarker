use time::OffsetDateTime;
use uuid::Uuid;

use crate::auth::UserContext;
use crate::domain::{ArchiveStatus, InboxStatus, ItemKind, SubmittedUrl, WatchStatus};
use crate::error::AppResult;
use crate::library::{
    CaptureItemOutcome, CaptureItemRequest, ItemTag, ItemUrlSummary, LibraryItemDetail,
    LibraryItemSummary,
};
use crate::url_normalization::{normalize_url_with_resolver, NormalizedUrl};

use super::{
    clean_optional, tag_ops, validate_client_capture_id, validation_error, InMemoryLibraryService,
};

pub(super) async fn capture_url(
    service: &InMemoryLibraryService,
    user: &UserContext,
    request: CaptureItemRequest,
) -> AppResult<CaptureItemOutcome> {
    let original_url = SubmittedUrl::new(request.url)
        .map_err(validation_error)?
        .into_string();
    let client_capture_id = validate_client_capture_id(request.client_capture_id)?;
    if let Some(item) = service.existing_capture(user, client_capture_id.as_deref())? {
        return Ok(existing_outcome(item));
    }

    let normalized_url =
        normalize_url_with_resolver(&original_url, service.url_resolver.as_ref()).await;
    if let Some(item) =
        existing_canonical_capture(service, user, normalized_url.canonical_url.as_deref())
    {
        return Ok(existing_outcome(item));
    }

    let tags = tag_ops::capture_tags(&service.tags_by_user, &user.sub, &request.tags)?;
    let item = new_capture_item(
        original_url,
        normalized_url,
        clean_optional(request.title),
        tags,
    );
    service.store_capture(user, client_capture_id, item.clone());
    Ok(CaptureItemOutcome {
        item,
        created: true,
    })
}

fn existing_canonical_capture(
    service: &InMemoryLibraryService,
    user: &UserContext,
    canonical_url: Option<&str>,
) -> Option<LibraryItemDetail> {
    let canonical_url = canonical_url?;
    service
        .items_by_user
        .lock()
        .unwrap()
        .get(&user.sub)
        .and_then(|items| {
            items
                .iter()
                .find(|item| {
                    item.summary
                        .url
                        .as_ref()
                        .and_then(|url| url.canonical_url.as_deref())
                        == Some(canonical_url)
                })
                .cloned()
        })
}

fn new_capture_item(
    original_url: String,
    normalized_url: NormalizedUrl,
    title: Option<String>,
    tags: Vec<ItemTag>,
) -> LibraryItemDetail {
    let NormalizedUrl { canonical_url, .. } = normalized_url;
    LibraryItemDetail {
        summary: LibraryItemSummary {
            id: Uuid::new_v4(),
            item_kind: ItemKind::Url,
            url: Some(ItemUrlSummary::new(original_url, canonical_url)),
            text: None,
            image: None,
            title,
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

fn existing_outcome(item: LibraryItemDetail) -> CaptureItemOutcome {
    CaptureItemOutcome {
        item,
        created: false,
    }
}
