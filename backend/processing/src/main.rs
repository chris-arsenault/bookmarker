use ahara_lambda_telemetry::{Operation, TelemetryConfig};
use aws_config::BehaviorVersion;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use processing::extractors::{OpenGraphExtractor, ReqwestMetadataFetch};
use processing::snapshot_store::{ReqwestThumbnailDownloader, S3ThumbnailStore};
use processing::ProcessingPipeline;
use serde::{Deserialize, Serialize};
use shared::config::AppConfig;
use shared::db::connect_pool;
use shared::processing::ProcessingRepository;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ProcessEvent {
    item_id: Option<Uuid>,
}

impl ProcessEvent {
    fn required_item_id(&self) -> Result<Uuid, ProcessingEventError> {
        self.item_id.ok_or(ProcessingEventError::MissingItemId)
    }
}

#[derive(Debug, thiserror::Error)]
enum ProcessingEventError {
    #[error("processing event requires item_id")]
    MissingItemId,
}

async fn handler(event: LambdaEvent<ProcessEvent>) -> Result<(), Error> {
    let item_id = event.payload.required_item_id()?;
    Operation::new(
        TelemetryConfig::new("linkdrop-processing"),
        "processing.process_item",
    )
    .with_domain("processing")
    .observe(async {
        runtime_pipeline()
            .await?
            .process_item(item_id)
            .await
            .map_err(|err| -> Error { Box::new(err) })
    })
    .await?;
    Ok(())
}

async fn runtime_pipeline() -> Result<
    ProcessingPipeline<
        OpenGraphExtractor<ReqwestMetadataFetch>,
        S3ThumbnailStore<ReqwestThumbnailDownloader>,
    >,
    Error,
> {
    let app_config = AppConfig::from_env()?;
    let db = connect_pool(&app_config).await?;
    let aws_config = aws_config::load_defaults(BehaviorVersion::latest()).await;
    let s3 = aws_sdk_s3::Client::new(&aws_config);
    let snapshot_bucket = std::env::var("SNAPSHOT_BUCKET")?;
    let http = reqwest::Client::new();
    Ok(ProcessingPipeline::new(
        ProcessingRepository::new(db),
        OpenGraphExtractor::new(ReqwestMetadataFetch::new(http.clone())),
        S3ThumbnailStore::new(s3, snapshot_bucket, ReqwestThumbnailDownloader::new(http)),
        "lambda",
    ))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let telemetry = TelemetryConfig::new("linkdrop-processing");
    ahara_lambda_telemetry::init_lambda_logging(&telemetry);
    ahara_lambda_telemetry::run_event_lambda(telemetry, service_fn(handler)).await
}

#[cfg(test)]
mod tests {
    use super::{ProcessEvent, ProcessingEventError};

    #[test]
    fn processing_event_requires_item_id() {
        let err = ProcessEvent { item_id: None }
            .required_item_id()
            .unwrap_err();

        assert!(matches!(err, ProcessingEventError::MissingItemId));
    }
}
