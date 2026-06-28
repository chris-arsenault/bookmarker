#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    let telemetry = ahara_lambda_telemetry::TelemetryConfig::new("linkdrop-api");
    ahara_lambda_telemetry::init_lambda_logging(&telemetry);
    tracing::info!("api starting");
    let state = api::ApiState::from_env()
        .await
        .map_err(|err| lambda_http::Error::from(err.to_string()))?;
    ahara_lambda_telemetry::run_http_lambda(telemetry, api::router(state)).await
}
