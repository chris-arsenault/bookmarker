use time::OffsetDateTime;

use crate::domain::{ArchiveStatus, InboxStatus, WatchStatus};

use super::{LibraryItemDetail, LibraryItemSummary, ListItemsQuery};

pub(super) fn item_matches_query(item: &LibraryItemDetail, query: &ListItemsQuery) -> bool {
    matches_platform(&item.summary, query.platform.as_deref())
        && matches_tag(&item.summary, query.tag.as_deref())
        && matches_created_from(&item.summary, query.created_from)
        && matches_created_to(&item.summary, query.created_to)
        && matches_archive_status(&item.summary, query.archive_status)
        && matches_watch_status(&item.summary, query.watch_status)
        && matches_inbox_status(&item.summary, query.inbox_status)
        && matches_text(item, query.q.as_deref())
}

fn matches_platform(item: &LibraryItemSummary, platform: Option<&str>) -> bool {
    let Some(platform) = normalized_filter(platform) else {
        return true;
    };
    item.platform
        .as_deref()
        .map(|value| value.eq_ignore_ascii_case(&platform))
        .unwrap_or(false)
}

fn matches_tag(item: &LibraryItemSummary, tag: Option<&str>) -> bool {
    let Some(tag) = normalized_filter(tag) else {
        return true;
    };
    item.tags.iter().any(|item_tag| {
        item_tag.normalized_name == tag || item_tag.display_name.eq_ignore_ascii_case(&tag)
    })
}

fn matches_created_from(item: &LibraryItemSummary, created_from: Option<OffsetDateTime>) -> bool {
    created_from
        .map(|created_from| item.created_at >= created_from)
        .unwrap_or(true)
}

fn matches_created_to(item: &LibraryItemSummary, created_to: Option<OffsetDateTime>) -> bool {
    created_to
        .map(|created_to| item.created_at <= created_to)
        .unwrap_or(true)
}

fn matches_archive_status(
    item: &LibraryItemSummary,
    archive_status: Option<ArchiveStatus>,
) -> bool {
    archive_status
        .map(|archive_status| item.archive_status == archive_status)
        .unwrap_or(true)
}

fn matches_watch_status(item: &LibraryItemSummary, watch_status: Option<WatchStatus>) -> bool {
    watch_status
        .map(|watch_status| item.watch_status == watch_status)
        .unwrap_or(true)
}

fn matches_inbox_status(item: &LibraryItemSummary, inbox_status: Option<InboxStatus>) -> bool {
    inbox_status
        .map(|inbox_status| item.inbox_status == inbox_status)
        .unwrap_or(true)
}

fn matches_text(item: &LibraryItemDetail, query: Option<&str>) -> bool {
    let Some(query) = normalized_filter(query) else {
        return true;
    };
    contains_text(item.summary.title.as_deref(), &query)
        || contains_text(item.summary.fetched_title.as_deref(), &query)
        || contains_text(
            item.summary
                .text
                .as_ref()
                .map(|text| text.plain_text.as_str()),
            &query,
        )
        || contains_text(Some(&item.notes), &query)
}

fn contains_text(value: Option<&str>, query: &str) -> bool {
    value
        .map(|value| value.to_ascii_lowercase().contains(query))
        .unwrap_or(false)
}

fn normalized_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
}
