use std::sync::Arc;

mod cors;
pub mod image_access;
mod item_routes;
pub mod processing_dispatch;
mod tag_routes;
pub mod thumbnail_access;

use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use cors::cors_layer;
use serde_json::{json, Value};
use shared::auth::{AuthVerifier, CognitoJwtVerifier, UserContext};
use shared::config::AppConfig;
use shared::db::{connect_pool, DbPool};
use shared::error::{AppError, AppResult};
use shared::library::LibraryService;
use shared::library_pg::PgLibraryService;

use image_access::{ImageObjectStore, S3ImageObjectStore};
use processing_dispatch::{LambdaProcessingDispatcher, ProcessingDispatcher};
use thumbnail_access::{S3ThumbnailReader, ThumbnailReader};

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
                auth: Arc::new(CognitoJwtVerifier::from_config(&config.cognito)),
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

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/me", get(me))
        .merge(item_routes::router())
        .merge(tag_routes::router())
        .layer(cors_layer())
        .with_state(state)
}

async fn health() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": shared::service_name(),
    }))
}

async fn me(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<UserContext>, ApiError> {
    Ok(Json(require_user(&state, &headers).await?))
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

pub struct ApiError(AppError);

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let public = self.0.public_error();
        let status =
            StatusCode::from_u16(public.status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        (
            status,
            Json(json!({
                "code": public.code,
                "message": public.message,
            })),
        )
            .into_response()
    }
}
