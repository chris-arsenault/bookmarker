use std::sync::Arc;

mod http;
pub mod image_access;
mod item_operation_details;
mod item_query;
mod item_routes;
pub mod processing_dispatch;
mod tag_routes;
pub mod thumbnail_access;

use ahara_lambda_telemetry::{Operation, OperationKind, TelemetryConfig};
use lambda_http::{Body, Request, Response};
use serde_json::json;
use shared::auth::{AlbValidatedJwtVerifier, AuthVerifier, UserContext};
use shared::config::AppConfig;
use shared::db::{connect_pool, DbPool};
use shared::error::{AppError, AppResult};
use shared::library::LibraryService;
use shared::library_pg::PgLibraryService;
use uuid::Uuid;

use http::prelude::*;
use http::{default_cors, error_response, HttpError, PublicHttpError};
use image_access::{ImageObjectStore, S3ImageObjectStore};
use processing_dispatch::{LambdaProcessingDispatcher, ProcessingDispatcher};
use thumbnail_access::{S3ThumbnailReader, ThumbnailReader};

pub type ApiResponse = Response<Body>;
pub type ApiResult<T> = Result<T, ApiError>;

pub fn http_body_bytes(body: &Body) -> &[u8] {
    http::body_bytes(body)
}

#[derive(Clone)]
pub struct ApiState {
    pub config: AppConfig,
    pub db: DbPool,
    pub auth: Arc<dyn AuthVerifier>,
    pub library: Arc<dyn LibraryService>,
    pub processing_dispatcher: Arc<dyn ProcessingDispatcher>,
    pub thumbnail_reader: Arc<dyn ThumbnailReader>,
    pub image_store: Arc<dyn ImageObjectStore>,
}

pub struct ApiStateServices {
    pub auth: Arc<dyn AuthVerifier>,
    pub library: Arc<dyn LibraryService>,
    pub processing_dispatcher: Arc<dyn ProcessingDispatcher>,
    pub thumbnail_reader: Arc<dyn ThumbnailReader>,
    pub image_store: Arc<dyn ImageObjectStore>,
}

impl ApiState {
    pub async fn from_env() -> AppResult<Self> {
        let config = AppConfig::from_env()?;
        let db = connect_pool(&config).await?;
        Ok(Self::new(
            config.clone(),
            db.clone(),
            ApiStateServices {
                auth: Arc::new(AlbValidatedJwtVerifier::new()),
                library: Arc::new(PgLibraryService::new(db.clone())),
                processing_dispatcher: Arc::new(
                    LambdaProcessingDispatcher::from_env(db.clone()).await,
                ),
                thumbnail_reader: Arc::new(S3ThumbnailReader::from_env().await?),
                image_store: Arc::new(S3ImageObjectStore::from_env().await?),
            },
        ))
    }

    pub fn new(config: AppConfig, db: DbPool, services: ApiStateServices) -> Self {
        Self {
            config,
            db,
            auth: services.auth,
            library: services.library,
            processing_dispatcher: services.processing_dispatcher,
            thumbnail_reader: services.thumbnail_reader,
            image_store: services.image_store,
        }
    }
}

pub async fn handle_request(request: Request, state: Arc<ApiState>) -> ApiResponse {
    let response = dispatch_request(&request, &state)
        .await
        .unwrap_or_else(|err| error_response(&err));
    default_cors(response)
}

async fn dispatch_request(request: &Request, state: &ApiState) -> ApiResult<ApiResponse> {
    let route = Route::from_request(request);
    if route.is_match(Method::GET, "/health")? {
        return health();
    }
    if route.is_match(Method::GET, "/me")? {
        return me(state, request.headers()).await;
    }
    if let Some(response) = item_routes::dispatch(&route, request, state).await? {
        return Ok(response);
    }
    if let Some(response) = tag_routes::dispatch(&route, request, state).await? {
        return Ok(response);
    }
    Err(HttpError::not_found().into())
}

fn health() -> ApiResult<ApiResponse> {
    Ok(json_value_response(
        StatusCode::OK,
        json!({
        "status": "ok",
        "service": shared::service_name(),
        }),
    ))
}

async fn me(state: &ApiState, headers: &HeaderMap) -> ApiResult<ApiResponse> {
    let user = require_user(state, headers).await?;
    observe_api_operation(user_api_operation("api.me", &user), async {
        json_response(StatusCode::OK, &user).map_err(Into::into)
    })
    .await
}

pub(crate) async fn require_user(
    state: &ApiState,
    headers: &HeaderMap,
) -> Result<UserContext, AppError> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    state
        .auth
        .context_from_authorization(auth_header.as_deref())
        .await
}

pub(crate) async fn observe_api_operation<T, E, Fut>(
    operation: Operation,
    future: Fut,
) -> Result<T, E>
where
    E: std::fmt::Debug,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    operation.observe(future).await
}

pub(crate) fn api_operation(name: &'static str) -> Operation {
    Operation::new(TelemetryConfig::new("linkdrop-api"), name).with_domain("api")
}

pub(crate) fn user_api_operation(name: &'static str, user: &UserContext) -> Operation {
    api_operation(name)
        .with_kind(OperationKind::UserInteraction)
        .with_detail("actor.kind", "authenticated_user")
        .with_detail("actor.label", user_label(user))
}

fn user_label(user: &UserContext) -> String {
    user.username
        .clone()
        .or_else(|| user.email.clone())
        .unwrap_or_else(|| "authenticated-user".to_string())
}

pub(crate) fn short_uuid_ref(id: Uuid) -> String {
    id.to_string().chars().take(8).collect()
}

#[derive(Debug, Clone)]
pub struct ApiError {
    status_code: StatusCode,
    code: String,
    message: String,
}

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        let public = value.public_error();
        Self {
            status_code: StatusCode::from_u16(public.status_code)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            code: public.code.to_string(),
            message: public.message,
        }
    }
}

impl From<HttpError> for ApiError {
    fn from(value: HttpError) -> Self {
        Self {
            status_code: value.status_code(),
            code: value.code().into_owned(),
            message: value.message().into_owned(),
        }
    }
}

impl PublicHttpError for ApiError {
    fn status_code(&self) -> StatusCode {
        self.status_code
    }

    fn code(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self.code)
    }

    fn message(&self) -> std::borrow::Cow<'_, str> {
        std::borrow::Cow::Borrowed(&self.message)
    }
}
