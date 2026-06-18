use uuid::Uuid;

use crate::auth::UserContext;
use crate::error::AppResult;

use super::{tag_ops, InMemoryLibraryService};

pub(super) fn delete_item(
    service: &InMemoryLibraryService,
    user: &UserContext,
    item_id: Uuid,
) -> AppResult<()> {
    let removed = remove_user_item(service, user, item_id)?;
    tag_ops::forget_item_tags(&service.tags_by_user, &user.sub, &removed.summary.tags);
    remove_capture_ids(service, &user.sub, item_id);
    Ok(())
}

fn remove_user_item(
    service: &InMemoryLibraryService,
    user: &UserContext,
    item_id: Uuid,
) -> AppResult<crate::library::LibraryItemDetail> {
    let mut items = service.items_by_user.lock().unwrap();
    let user_items = items
        .get_mut(&user.sub)
        .ok_or_else(|| super::not_found(item_id))?;
    let index = user_items
        .iter()
        .position(|item| item.summary.id == item_id)
        .ok_or_else(|| super::not_found(item_id))?;
    Ok(user_items.remove(index))
}

fn remove_capture_ids(service: &InMemoryLibraryService, user_sub: &str, item_id: Uuid) {
    if let Some(captures) = service
        .capture_ids_by_user
        .lock()
        .unwrap()
        .get_mut(user_sub)
    {
        captures.retain(|_, captured_item_id| *captured_item_id != item_id);
    }
}
