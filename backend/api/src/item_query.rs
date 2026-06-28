use serde::Deserialize;
use shared::domain::{ArchiveStatus, InboxStatus, WatchStatus};
use shared::library::{ListItemUpdatesQuery, ListItemsQuery};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::ApiError;

#[derive(Debug, Default, Deserialize)]
pub(crate) struct ListItemsParams {
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
pub(crate) struct ListItemUpdatesParams {
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
    pub(crate) fn into_query(self) -> Result<ListItemsQuery, ApiError> {
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
    pub(crate) fn into_query(self) -> Result<ListItemUpdatesQuery, ApiError> {
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

fn clean_param(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn validation_error(err: impl ToString) -> shared::error::AppError {
    shared::error::AppError::Validation(err.to_string())
}
