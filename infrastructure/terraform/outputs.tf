output "frontend_url" {
  value = module.frontend.url
}

output "api_url" {
  value = "https://${local.api_hostname}"
}

output "reserved_alb_priorities" {
  value = local.alb_priorities
}

output "snapshot_bucket_name" {
  value = aws_s3_bucket.snapshots.id
}

output "snapshot_bucket_arn" {
  value = aws_s3_bucket.snapshots.arn
}

output "cognito_client_id" {
  value = module.cognito_app.client_id
}

output "api_function_name" {
  value = module.api.function_names["api"]
}

output "processing_function_name" {
  value = module.processing.function_name
}

output "frontend_bucket_name" {
  value = module.frontend.bucket_name
}

output "frontend_distribution_id" {
  value = module.frontend.distribution_id
}

output "frontend_distribution_domain_name" {
  value = module.frontend.distribution_domain_name
}

output "lambda_error_alarm_names" {
  value = {
    for key, alarm in aws_cloudwatch_metric_alarm.lambda_errors :
    key => alarm.alarm_name
  }
}

output "lambda_throttle_alarm_names" {
  value = {
    for key, alarm in aws_cloudwatch_metric_alarm.lambda_throttles :
    key => alarm.alarm_name
  }
}
