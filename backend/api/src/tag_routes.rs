use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use shared::library::{MergeTagsRequest, RenameTagRequest, TagCorpusEntry};
use uuid::Uuid;

use crate::{require_user, ApiError, ApiState};

pub fn router() -> Router<ApiState> {
    Router::new()
        .route("/tags", get(list_tags))
        .route("/tags/{tag_id}", patch(rename_tag))
        .route("/tags/{source_tag_id}/merge", post(merge_tags))
}

async fn list_tags(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<Vec<TagCorpusEntry>>, ApiError> {
    let user = require_user(&state, &headers).await?;
    Ok(Json(state.library.list_tag_corpus(&user).await?))
}

async fn rename_tag(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(tag_id): Path<Uuid>,
    Json(request): Json<RenameTagRequest>,
) -> Result<Json<Vec<TagCorpusEntry>>, ApiError> {
    let user = require_user(&state, &headers).await?;
    Ok(Json(
        state.library.rename_tag(&user, tag_id, request).await?,
    ))
}

async fn merge_tags(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Path(source_tag_id): Path<Uuid>,
    Json(request): Json<MergeTagsRequest>,
) -> Result<Json<Vec<TagCorpusEntry>>, ApiError> {
    let user = require_user(&state, &headers).await?;
    Ok(Json(
        state
            .library
            .merge_tags(&user, source_tag_id, request)
            .await?,
    ))
}
