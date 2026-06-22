use uuid::Uuid;

use crate::auth::UserContext;
use crate::error::AppResult;
use crate::library::{LibraryItemDetail, LibraryItemSummary, UpdateItemRequest};

use super::{clean_optional, not_found, tag_ops, InMemoryLibraryService};

pub(super) fn update_item(
    service: &InMemoryLibraryService,
    user: &UserContext,
    item_id: Uuid,
    request: UpdateItemRequest,
) -> AppResult<LibraryItemDetail> {
    let mut items = service.items_by_user.lock().unwrap();
    let user_items = items.get_mut(&user.sub).ok_or_else(|| not_found(item_id))?;
    let item = user_items
        .iter_mut()
        .find(|item| item.summary.id == item_id)
        .ok_or_else(|| not_found(item_id))?;
    if let Some(title) = request.title {
        item.summary.title = clean_optional(Some(title));
    }
    if let Some(watch_status) = request.watch_status {
        item.summary.watch_status = watch_status;
    }
    if let Some(inbox_status) = request.inbox_status {
        item.summary.inbox_status = inbox_status;
    }
    if let Some(notes) = request.notes {
        item.notes = notes;
    }
    if let Some(tags) = request.tags {
        item.summary.tags = tag_ops::replace_item_tags(
            &service.tags_by_user,
            &user.sub,
            &item.summary.tags,
            &tags,
        )?;
    }
    Ok(item.clone())
}

pub(super) fn limited_cursor(
    default_cursor: time::OffsetDateTime,
    limit: i64,
    items: &[LibraryItemSummary],
) -> time::OffsetDateTime {
    if items.len() >= limit as usize {
        items
            .last()
            .map(|item| item.created_at)
            .unwrap_or(default_cursor)
    } else {
        default_cursor
    }
}
