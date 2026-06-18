use crate::domain::{ArchiveStatus, InboxStatus, WatchStatus};
use crate::library::ListItemsQuery;

pub(super) struct PgListFilters {
    pub(super) platform: Option<String>,
    pub(super) tag: Option<String>,
    pub(super) archive_status: Option<&'static str>,
    pub(super) watch_status: Option<&'static str>,
    pub(super) inbox_status: Option<&'static str>,
    pub(super) q: Option<String>,
}

impl From<&ListItemsQuery> for PgListFilters {
    fn from(query: &ListItemsQuery) -> Self {
        Self {
            platform: normalized_filter(query.platform.as_deref()),
            tag: normalized_filter(query.tag.as_deref()),
            archive_status: query.archive_status.map(ArchiveStatus::as_str),
            watch_status: query.watch_status.map(WatchStatus::as_str),
            inbox_status: query.inbox_status.map(InboxStatus::as_str),
            q: normalized_filter(query.q.as_deref()),
        }
    }
}

fn normalized_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
}
