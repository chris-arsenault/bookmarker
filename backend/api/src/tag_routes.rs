use crate::http::prelude::*;
use shared::library::{MergeTagsRequest, RenameTagRequest};
use uuid::Uuid;

use crate::{
    observe_api_operation, require_user, user_api_operation, ApiResponse, ApiResult, ApiState,
};

pub async fn dispatch(
    route: &Route<'_>,
    request: &Request,
    state: &ApiState,
) -> ApiResult<Option<ApiResponse>> {
    if route.is_match(Method::GET, "/tags")? {
        return list_tags(state, request).await.map(Some);
    }
    if let Some(params) = route.matches(Method::PATCH, "/tags/{tag_id}")? {
        return rename_tag(state, request, params.parse("tag_id")?)
            .await
            .map(Some);
    }
    if let Some(params) = route.matches(Method::POST, "/tags/{source_tag_id}/merge")? {
        return merge_tags(state, request, params.parse("source_tag_id")?)
            .await
            .map(Some);
    }
    Ok(None)
}

async fn list_tags(state: &ApiState, request: &Request) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    observe_api_operation(user_api_operation("api.tags.list", &user), async {
        json_response(StatusCode::OK, &state.library.list_tag_corpus(&user).await?)
            .map_err(Into::into)
    })
    .await
}

async fn rename_tag(state: &ApiState, request: &Request, tag_id: Uuid) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    let rename = json_body::<RenameTagRequest>(request)?;
    let operation =
        user_api_operation("api.tags.rename", &user).with_detail("tag.id", tag_id.to_string());
    observe_api_operation(operation, async {
        json_response(
            StatusCode::OK,
            &state.library.rename_tag(&user, tag_id, rename).await?,
        )
        .map_err(Into::into)
    })
    .await
}

async fn merge_tags(
    state: &ApiState,
    request: &Request,
    source_tag_id: Uuid,
) -> ApiResult<ApiResponse> {
    let user = require_user(state, request.headers()).await?;
    let merge = json_body::<MergeTagsRequest>(request)?;
    let operation = user_api_operation("api.tags.merge", &user)
        .with_detail("tag.source_id", source_tag_id.to_string())
        .with_detail("tag.target_id", merge.target_tag_id.to_string());
    observe_api_operation(operation, async {
        json_response(
            StatusCode::OK,
            &state
                .library
                .merge_tags(&user, source_tag_id, merge)
                .await?,
        )
        .map_err(Into::into)
    })
    .await
}
