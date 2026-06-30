locals {
  aws_region  = "us-east-1"
  prefix      = "linkdrop"
  domain_name = "ahara.io"

  frontend_hostname = "linkdrop.${local.domain_name}"
  api_hostname      = "api.linkdrop.${local.domain_name}"

  snapshot_bucket = "${local.prefix}-snapshots"
  snapshot_prefix = "snapshots/"
  product_name    = "Linkdrop"

  frontend_description       = "Save videos and links from any app, then browse and organize them across Ahara surfaces."
  frontend_social_image_path = "/social-card.png"

  db_env = {
    DB_HOST     = module.ctx.rds.address
    DB_PORT     = module.ctx.rds.port
    DB_NAME     = nonsensitive(data.aws_ssm_parameter.db_database.value)
    DB_USERNAME = nonsensitive(data.aws_ssm_parameter.db_username.value)
    DB_PASSWORD = nonsensitive(data.aws_ssm_parameter.db_password.value)
  }

  otel_env = {
    OTEL_EXPORTER_OTLP_ENDPOINT = nonsensitive(data.aws_ssm_parameter.observability_otlp_http_endpoint.value)
    OTEL_LOGS_EXPORTER          = "otlp"
    OTEL_METRICS_EXPORTER       = "otlp"
    OTEL_TRACES_EXPORTER        = "otlp"
  }

  common_env = merge(local.db_env, local.otel_env, {
    API_BASE_URL           = "https://${local.api_hostname}"
    APP_BASE_URL           = "https://${local.frontend_hostname}"
    COGNITO_USER_POOL_ID   = module.ctx.cognito.user_pool_id
    COGNITO_CLIENT_ID      = module.cognito_app.client_id
    COGNITO_DOMAIN         = module.ctx.cognito.domain
    COGNITO_ISSUER         = module.ctx.cognito.issuer
    DEPLOYMENT_ENVIRONMENT = "production"
  })

  alb_priorities = {
    api_health        = 380
    api_authenticated = 381
  }

  lambda_reserved_concurrency = {
    api        = 10
    processing = 2
  }

  lambda_memory_size = {
    processing = 512
  }

  lambda_timeout_seconds = {
    processing = 300
  }

  default_tags = {
    Project   = local.prefix
    ManagedBy = "Terraform"
  }
}
