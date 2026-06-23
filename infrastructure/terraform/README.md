# Terraform

Terraform root for Linkdrop.

The root defines backend/provider settings and deploys Linkdrop through shared
Ahara modules:

| File                  | Purpose                                                                                                                    |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------- |
| `context.tf`          | Reads shared VPC, ALB, Cognito, RDS, and Route 53 context through `platform-context`.                                      |
| `ssm.tf`              | Reads per-project database credentials from `/ahara/db/linkdrop/*`.                                                        |
| `cognito.tf`          | Creates the `linkdrop-app` Cognito client and `/ahara/auth-trigger/clients/linkdrop-app` SSM mapping.                      |
| `lambdas.tf`          | Deploys the ALB API Lambda and async processing Lambda with `PROCESSING_FUNCTION_NAME` and snapshot bucket runtime config. |
| `frontend.tf`         | Deploys the React site and injects runtime config for API URL, app URL, product name, and Cognito IDs.                     |
| `snapshot_storage.tf` | Creates the private, versioned Linkdrop snapshot bucket.                                                                   |
| `alarms.tf`           | Creates CloudWatch `Errors` and `Throttles` alarms for the API and processing Lambdas.                                     |

Plan with credentials:

```bash
with-cred -- terraform -chdir=infrastructure/terraform plan -refresh=false -input=false -out=/tmp/linkdrop-m8.tfplan
```

This root consumes the shared Ahara VPC, ALB, RDS, Cognito user pool, NAT
gateway, API routing, and Terraform state bucket resources through platform
context and reusable modules.
