use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::UserContext;
use crate::domain::{ItemKind, SubmittedUrl, TagName, TextSnippetBody};
use crate::error::{AppError, AppResult};
use crate::library::{CaptureItemOutcome, CaptureItemRequest, CaptureTextRequest, LibraryService};
use crate::library_pg_capture_helpers::{
    validate_client_capture_id, validate_tags, validation_error,
};
use crate::url_normalization::{normalize_url_with_resolver, NormalizedUrl};

use super::UPSERT_TAG;
use super::{canonical_conflict_without_row, database_error, PgLibraryService};
use super::{GET_ITEM_BY_TEXT_HASH, INSERT_CAPTURE_ITEM, INSERT_ITEM_TAG, INSERT_TEXT_CAPTURE};

pub(super) async fn capture_url(
    service: &PgLibraryService,
    user: &UserContext,
    request: CaptureItemRequest,
) -> AppResult<CaptureItemOutcome> {
    let original_url = SubmittedUrl::new(&request.url)
        .map_err(validation_error)?
        .into_string();
    let client_capture_id = validate_client_capture_id(request.client_capture_id.clone())?;
    let user_id = service.upsert_user(user).await?;
    if let Some(item) = service
        .existing_capture(user_id, client_capture_id.as_deref())
        .await?
    {
        return Ok(existing_outcome(item));
    }

    let normalized_url =
        normalize_url_with_resolver(&original_url, service.url_resolver.as_ref()).await;
    if let Some(item) = service
        .existing_canonical_capture(user_id, normalized_url.canonical_url.as_deref())
        .await?
    {
        return Ok(existing_outcome(item));
    }

    insert_url_capture(
        service,
        user,
        UrlCaptureInsert {
            user_id,
            original_url,
            normalized_url,
            client_capture_id,
            tags: request.tags,
        },
    )
    .await
}

struct UrlCaptureInsert {
    user_id: Uuid,
    original_url: String,
    normalized_url: NormalizedUrl,
    client_capture_id: Option<String>,
    tags: Vec<String>,
}

async fn insert_url_capture(
    service: &PgLibraryService,
    user: &UserContext,
    input: UrlCaptureInsert,
) -> AppResult<CaptureItemOutcome> {
    let tags = validate_tags(&input.tags)?;
    let item_id = service
        .insert_capture(
            input.user_id,
            &input.original_url,
            &input.normalized_url,
            input.client_capture_id.as_deref(),
            &tags,
        )
        .await?;
    let Some(item_id) = item_id else {
        let item = service
            .existing_canonical_capture(
                input.user_id,
                input.normalized_url.canonical_url.as_deref(),
            )
            .await?
            .ok_or_else(canonical_conflict_without_row)?;
        return Ok(existing_outcome(item));
    };
    Ok(CaptureItemOutcome {
        item: service.get_item(user, item_id).await?,
        created: true,
    })
}

pub(super) async fn capture_text(
    service: &PgLibraryService,
    user: &UserContext,
    request: CaptureTextRequest,
) -> AppResult<CaptureItemOutcome> {
    let plain_text = TextSnippetBody::new(request.plain_text.clone())
        .map_err(validation_error)?
        .into_string();
    let client_capture_id = validate_client_capture_id(request.client_capture_id.clone())?;
    let user_id = service.upsert_user(user).await?;
    if let Some(item) = service
        .existing_capture(user_id, client_capture_id.as_deref())
        .await?
    {
        return Ok(existing_outcome(item));
    }

    let content_hash = content_hash(&plain_text, request.html.as_deref());
    if let Some(item) = existing_text_capture(service, user_id, &content_hash).await? {
        return Ok(existing_outcome(item));
    }

    insert_text_capture(
        service,
        user,
        TextCaptureInsert {
            user_id,
            plain_text,
            content_hash,
            client_capture_id,
            request,
        },
    )
    .await
}

struct TextCaptureInsert {
    user_id: Uuid,
    plain_text: String,
    content_hash: String,
    client_capture_id: Option<String>,
    request: CaptureTextRequest,
}

async fn existing_text_capture(
    service: &PgLibraryService,
    user_id: Uuid,
    content_hash: &str,
) -> AppResult<Option<crate::library::LibraryItemDetail>> {
    let row = sqlx::query_as(&format!("{}{}", super::ITEM_SELECT, GET_ITEM_BY_TEXT_HASH))
        .bind(user_id)
        .bind(content_hash)
        .fetch_optional(&service.db)
        .await
        .map_err(database_error)?;
    match row {
        Some(row) => Ok(Some(service.detail_from_row(row).await?)),
        None => Ok(None),
    }
}

async fn insert_text_capture(
    service: &PgLibraryService,
    user: &UserContext,
    input: TextCaptureInsert,
) -> AppResult<CaptureItemOutcome> {
    let tags = validate_tags(&input.request.tags)?;
    let mut transaction = service.db.begin().await.map_err(database_error)?;
    let item_id = insert_text_item(
        &mut transaction,
        input.user_id,
        input.client_capture_id.as_deref(),
    )
    .await?;
    let inserted = insert_text_payload(&mut transaction, item_id, input.user_id, &input).await?;
    if !inserted {
        transaction.rollback().await.map_err(database_error)?;
        return text_conflict(service, input.user_id, &input.content_hash).await;
    }
    attach_tags(&mut transaction, item_id, input.user_id, &tags).await?;
    transaction.commit().await.map_err(database_error)?;
    Ok(CaptureItemOutcome {
        item: service.get_item(user, item_id).await?,
        created: true,
    })
}

async fn insert_text_item(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    client_capture_id: Option<&str>,
) -> AppResult<Uuid> {
    sqlx::query_scalar(INSERT_CAPTURE_ITEM)
        .bind(user_id)
        .bind(client_capture_id)
        .bind(ItemKind::TextSnippet.as_str())
        .fetch_one(&mut **transaction)
        .await
        .map_err(database_error)
}

async fn insert_text_payload(
    transaction: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    user_id: Uuid,
    input: &TextCaptureInsert,
) -> AppResult<bool> {
    let result = sqlx::query(INSERT_TEXT_CAPTURE)
        .bind(item_id)
        .bind(user_id)
        .bind(&input.plain_text)
        .bind(clean_optional(input.request.html.clone()))
        .bind(&input.content_hash)
        .bind(clean_optional(input.request.source_app.clone()))
        .bind(clean_optional(input.request.source_device.clone()))
        .bind(capture_method(input.request.capture_method.clone()))
        .execute(&mut **transaction)
        .await
        .map_err(database_error)?;
    Ok(result.rows_affected() > 0)
}

async fn attach_tags(
    transaction: &mut Transaction<'_, Postgres>,
    item_id: Uuid,
    user_id: Uuid,
    tags: &[TagName],
) -> AppResult<()> {
    for tag in tags {
        let tag_id: Uuid = sqlx::query_scalar(UPSERT_TAG)
            .bind(user_id)
            .bind(tag.display_name())
            .fetch_one(&mut **transaction)
            .await
            .map_err(database_error)?;
        sqlx::query(INSERT_ITEM_TAG)
            .bind(item_id)
            .bind(tag_id)
            .bind(user_id)
            .execute(&mut **transaction)
            .await
            .map_err(database_error)?;
    }
    Ok(())
}

async fn text_conflict(
    service: &PgLibraryService,
    user_id: Uuid,
    content_hash: &str,
) -> AppResult<CaptureItemOutcome> {
    let item = existing_text_capture(service, user_id, content_hash)
        .await?
        .ok_or_else(|| AppError::Database("text content conflict did not return an item".into()))?;
    Ok(existing_outcome(item))
}

fn existing_outcome(item: crate::library::LibraryItemDetail) -> CaptureItemOutcome {
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
