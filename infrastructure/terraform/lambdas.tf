module "api" {
  source   = "git::https://github.com/chris-arsenault/ahara-tf-patterns.git//modules/alb-api"
  prefix   = local.prefix
  hostname = local.api_hostname

  vpc     = module.ctx.vpc
  alb     = module.ctx.alb
  cognito = module.ctx.cognito

  environment = local.common_env

  iam_policy = [jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect   = "Allow"
        Action   = ["s3:GetObject", "s3:PutObject"]
        Resource = "${aws_s3_bucket.snapshots.arn}/*"
      },
      {
        Effect   = "Allow"
        Action   = ["lambda:InvokeFunction"]
        Resource = module.processing.function_arn
      }
    ]
  })]

  lambdas = {
    api = {
      binary = "${path.module}/../../backend/target/lambda/api/bootstrap"
      routes = [
        {
          priority      = local.alb_priorities.api_health
          paths         = ["/health"]
          methods       = ["GET", "HEAD"]
          authenticated = false
        },
        {
          priority      = local.alb_priorities.api_authenticated
          paths         = ["/*"]
          authenticated = true
        }
      ]
      environment = {
        PROCESSING_FUNCTION_NAME = module.processing.function_name
        SNAPSHOT_BUCKET          = aws_s3_bucket.snapshots.id
      }
      reserved_concurrent_executions = local.lambda_reserved_concurrency.api
    }
  }
}

module "processing" {
  source = "git::https://github.com/chris-arsenault/ahara-tf-patterns.git//modules/lambda"

  name        = "${local.prefix}-processing"
  binary      = "${path.module}/../../backend/target/lambda/processing/bootstrap"
  role_arn    = module.api.role_arn
  timeout     = local.lambda_timeout_seconds.processing
  memory_size = local.lambda_memory_size.processing

  vpc = module.ctx.vpc

  environment = merge(local.common_env, {
    SNAPSHOT_BUCKET = aws_s3_bucket.snapshots.id
  })
}
