use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use uuid::Uuid;

use crate::domain::TagName;
use crate::error::{AppError, AppResult};

use super::{UserItems, UserTags};
use crate::library::{ItemTag, MergeTagsRequest, RenameTagRequest, TagCorpusEntry};

pub(super) fn capture_tags(
    tags_by_user: &Arc<Mutex<UserTags>>,
    user_sub: &str,
    tags: &[String],
) -> AppResult<Vec<ItemTag>> {
    let mut user_tags = tags_by_user.lock().unwrap();
    let user_tags = user_tags.entry(user_sub.to_string()).or_default();
    let selected = apply_tags(user_tags, &validate_tag_names(tags)?);
    sort_tag_corpus(user_tags);
    Ok(selected)
}

pub(super) fn replace_item_tags(
    tags_by_user: &Arc<Mutex<UserTags>>,
    user_sub: &str,
    current_tags: &[ItemTag],
    tags: &[String],
) -> AppResult<Vec<ItemTag>> {
    let tags = validate_tag_names(tags)?;
    let mut user_tags = tags_by_user.lock().unwrap();
    let user_tags = user_tags.entry(user_sub.to_string()).or_default();

    for current_tag in current_tags {
        decrement_tag(user_tags, &current_tag.normalized_name);
    }
    let selected = apply_tags(user_tags, &tags);
    user_tags.retain(|entry| entry.usage_count > 0);
    sort_tag_corpus(user_tags);
    Ok(selected)
}

pub(super) fn forget_item_tags(
    tags_by_user: &Arc<Mutex<UserTags>>,
    user_sub: &str,
    current_tags: &[ItemTag],
) {
    let mut tags_by_user = tags_by_user.lock().unwrap();
    let Some(user_tags) = tags_by_user.get_mut(user_sub) else {
        return;
    };
    for current_tag in current_tags {
        decrement_tag(user_tags, &current_tag.normalized_name);
    }
    user_tags.retain(|entry| entry.usage_count > 0);
    sort_tag_corpus(user_tags);
}

pub(super) fn rename_tag(
    items_by_user: &Arc<Mutex<UserItems>>,
    tags_by_user: &Arc<Mutex<UserTags>>,
    user_sub: &str,
    tag_id: Uuid,
    request: RenameTagRequest,
) -> AppResult<Vec<TagCorpusEntry>> {
    let tag = TagName::new(&request.display_name).map_err(validation_error)?;
    let mut items_by_user = items_by_user.lock().unwrap();
    let mut tags_by_user = tags_by_user.lock().unwrap();
    let user_tags = tags_by_user.entry(user_sub.to_string()).or_default();

    reject_rename_collision(user_tags, tag_id, tag.normalized_name())?;
    let entry = user_tags
        .iter_mut()
        .find(|entry| entry.id == tag_id)
        .ok_or_else(|| tag_not_found(tag_id))?;
    entry.display_name = tag.display_name().to_string();
    entry.normalized_name = tag.normalized_name().to_string();

    if let Some(items) = items_by_user.get_mut(user_sub) {
        rename_item_tags(items, tag_id, entry);
    }
    sort_tag_corpus(user_tags);
    Ok(user_tags.clone())
}

pub(super) fn merge_tags(
    items_by_user: &Arc<Mutex<UserItems>>,
    tags_by_user: &Arc<Mutex<UserTags>>,
    user_sub: &str,
    source_tag_id: Uuid,
    request: MergeTagsRequest,
) -> AppResult<Vec<TagCorpusEntry>> {
    if source_tag_id == request.target_tag_id {
        return Err(AppError::Validation(
            "cannot merge a tag into itself".to_string(),
        ));
    }

    let mut items_by_user = items_by_user.lock().unwrap();
    let mut tags_by_user = tags_by_user.lock().unwrap();
    let user_tags = tags_by_user.entry(user_sub.to_string()).or_default();
    let target = user_tags
        .iter()
        .find(|entry| entry.id == request.target_tag_id)
        .cloned()
        .ok_or_else(|| tag_not_found(request.target_tag_id))?;
    ensure_tag_exists(user_tags, source_tag_id)?;

    if let Some(items) = items_by_user.get_mut(user_sub) {
        merge_item_tags(items, source_tag_id, &target);
        recount_tag_usage(user_tags, items);
    }
    user_tags.retain(|entry| entry.id != source_tag_id && entry.usage_count > 0);
    sort_tag_corpus(user_tags);
    Ok(user_tags.clone())
}

fn apply_tag(user_tags: &mut Vec<TagCorpusEntry>, tag: &TagName) -> ItemTag {
    if let Some(entry) = user_tags
        .iter_mut()
        .find(|entry| entry.normalized_name == tag.normalized_name())
    {
        entry.usage_count += 1;
        return item_tag(entry);
    }

    let entry = TagCorpusEntry {
        id: Uuid::new_v4(),
        display_name: tag.display_name().to_string(),
        normalized_name: tag.normalized_name().to_string(),
        usage_count: 1,
    };
    let tag = item_tag(&entry);
    user_tags.push(entry);
    tag
}

fn apply_tags(user_tags: &mut Vec<TagCorpusEntry>, tags: &[TagName]) -> Vec<ItemTag> {
    tags.iter().map(|tag| apply_tag(user_tags, tag)).collect()
}

fn decrement_tag(user_tags: &mut [TagCorpusEntry], normalized_name: &str) {
    if let Some(entry) = user_tags
        .iter_mut()
        .find(|entry| entry.normalized_name == normalized_name)
    {
        entry.usage_count = entry.usage_count.saturating_sub(1);
    }
}

fn validate_tag_names(tags: &[String]) -> AppResult<Vec<TagName>> {
    let mut seen = HashSet::new();
    let mut validated = Vec::new();

    for value in tags {
        let tag = TagName::new(value).map_err(validation_error)?;
        if seen.insert(tag.normalized_name().to_string()) {
            validated.push(tag);
        }
    }

    Ok(validated)
}

fn reject_rename_collision(
    user_tags: &[TagCorpusEntry],
    tag_id: Uuid,
    normalized_name: &str,
) -> AppResult<()> {
    let collision = user_tags
        .iter()
        .any(|entry| entry.id != tag_id && entry.normalized_name == normalized_name);
    if collision {
        return Err(AppError::Validation("tag name already exists".to_string()));
    }
    Ok(())
}

fn ensure_tag_exists(user_tags: &[TagCorpusEntry], tag_id: Uuid) -> AppResult<()> {
    user_tags
        .iter()
        .any(|entry| entry.id == tag_id)
        .then_some(())
        .ok_or_else(|| tag_not_found(tag_id))
}

fn rename_item_tags(
    items: &mut [crate::library::LibraryItemDetail],
    tag_id: Uuid,
    entry: &TagCorpusEntry,
) {
    for item in items {
        for item_tag in &mut item.summary.tags {
            if item_tag.id == tag_id {
                *item_tag = item_tag_from_entry(entry);
            }
        }
    }
}

fn merge_item_tags(
    items: &mut [crate::library::LibraryItemDetail],
    source_tag_id: Uuid,
    target: &TagCorpusEntry,
) {
    for item in items {
        let had_source = item.summary.tags.iter().any(|tag| tag.id == source_tag_id);
        let has_target = item.summary.tags.iter().any(|tag| tag.id == target.id);
        item.summary.tags.retain(|tag| tag.id != source_tag_id);
        if had_source && !has_target {
            item.summary.tags.push(item_tag_from_entry(target));
        }
        item.summary
            .tags
            .sort_by(|left, right| left.normalized_name.cmp(&right.normalized_name));
    }
}

fn recount_tag_usage(
    user_tags: &mut [TagCorpusEntry],
    items: &[crate::library::LibraryItemDetail],
) {
    for entry in user_tags.iter_mut() {
        entry.usage_count = items
            .iter()
            .filter(|item| item.summary.tags.iter().any(|tag| tag.id == entry.id))
            .count() as i32;
    }
}

fn item_tag(entry: &TagCorpusEntry) -> ItemTag {
    item_tag_from_entry(entry)
}

fn item_tag_from_entry(entry: &TagCorpusEntry) -> ItemTag {
    ItemTag {
        id: entry.id,
        display_name: entry.display_name.clone(),
        normalized_name: entry.normalized_name.clone(),
    }
}

fn sort_tag_corpus(tags: &mut [TagCorpusEntry]) {
    tags.sort_by(|left, right| {
        right
            .usage_count
            .cmp(&left.usage_count)
            .then_with(|| left.normalized_name.cmp(&right.normalized_name))
    });
}

fn validation_error(err: impl ToString) -> AppError {
    AppError::Validation(err.to_string())
}

fn tag_not_found(tag_id: Uuid) -> AppError {
    AppError::NotFound(format!("tag {tag_id}"))
}
