#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .json()
        .init();
    tracing::info!("api starting");
    let state = api::ApiState::from_env()
        .await
        .map_err(|err| lambda_http::Error::from(err.to_string()))?;
    lambda_http::run(api::router(state)).await
}
