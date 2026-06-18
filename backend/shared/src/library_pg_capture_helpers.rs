use std::collections::HashSet;

use crate::domain::TagName;
use crate::error::{AppError, AppResult};

pub(crate) fn validate_client_capture_id(value: Option<String>) -> AppResult<Option<String>> {
    value
        .map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                Err(AppError::Validation(
                    "client_capture_id must not be empty".to_string(),
                ))
            } else {
                Ok(trimmed.to_string())
            }
        })
        .transpose()
}

pub(crate) fn validate_tags(values: &[String]) -> AppResult<Vec<TagName>> {
    let mut seen = HashSet::new();
    let mut tags = Vec::new();
    for value in values {
        let tag = TagName::new(value).map_err(validation_error)?;
        if seen.insert(tag.normalized_name().to_string()) {
            tags.push(tag);
        }
    }
    Ok(tags)
}

pub(crate) fn validation_error(err: impl ToString) -> AppError {
    AppError::Validation(err.to_string())
}
