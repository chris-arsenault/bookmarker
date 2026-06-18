module "frontend" {
  source = "git::https://github.com/chris-arsenault/ahara-tf-patterns.git//modules/website"

  prefix         = local.prefix
  hostname       = local.frontend_hostname
  site_directory = "${path.module}/../../frontend/dist"

  vpc = {
    private_subnet_ids = module.ctx.vpc.private_subnet_ids
    lambda_sg_id       = module.ctx.vpc.lambda_sg_id
  }

  og_artifact = module.ctx.og_server

  og_config = {
    site_name = local.product_name
    defaults = {
      title       = local.product_name
      description = local.frontend_description
      image       = local.frontend_social_image_path
    }
    routes = [
      {
        pattern     = "/"
        query       = ""
        title       = local.product_name
        description = local.frontend_description
        image       = local.frontend_social_image_path
        og_type     = "website"
      }
    ]
  }

  runtime_config = {
    apiBaseUrl        = "https://${local.api_hostname}"
    appBaseUrl        = "https://${local.frontend_hostname}"
    productName       = local.product_name
    cognitoUserPoolId = module.ctx.cognito.user_pool_id
    cognitoClientId   = module.cognito_app.client_id
  }
}
