use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_lambda::primitives::Blob;
use aws_sdk_lambda::types::InvocationType;
use serde_json::json;
use shared::db::DbPool;
use shared::domain::ProcessingJobKind;
use shared::error::{AppError, AppResult};
use shared::processing::ProcessingRepository;
use uuid::Uuid;

#[async_trait]
pub trait ProcessingDispatcher: Send + Sync {
    async fn dispatch_item(&self, item_id: Uuid) -> AppResult<()>;
}

#[derive(Clone)]
pub struct LambdaProcessingDispatcher {
    repository: ProcessingRepository,
    function_name: Option<String>,
    lambda: Option<aws_sdk_lambda::Client>,
}

impl LambdaProcessingDispatcher {
    pub async fn from_env(db: DbPool) -> Self {
        let function_name = processing_function_name();
        let lambda = if function_name.is_some() {
            let config = aws_config::load_defaults(BehaviorVersion::latest()).await;
            Some(aws_sdk_lambda::Client::new(&config))
        } else {
            None
        };
        Self {
            repository: ProcessingRepository::new(db),
            function_name,
            lambda,
        }
    }
}

#[async_trait]
impl ProcessingDispatcher for LambdaProcessingDispatcher {
    async fn dispatch_item(&self, item_id: Uuid) -> AppResult<()> {
        self.repository
            .enqueue_job(item_id, ProcessingJobKind::EnrichMetadata)
            .await?;
        let (Some(function_name), Some(lambda)) = (&self.function_name, &self.lambda) else {
            return Ok(());
        };
        lambda
            .invoke()
            .function_name(function_name)
            .invocation_type(InvocationType::Event)
            .payload(Blob::new(
                serde_json::to_vec(&json!({ "item_id": item_id })).unwrap_or_default(),
            ))
            .send()
            .await
            .map_err(|err| AppError::ExternalService {
                service: "lambda",
                message: err.to_string(),
            })?;
        Ok(())
    }
}

fn processing_function_name() -> Option<String> {
    std::env::var("PROCESSING_FUNCTION_NAME")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
