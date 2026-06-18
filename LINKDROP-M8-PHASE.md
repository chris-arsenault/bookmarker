# Linkdrop M8 Phase Plan - Deploy, Operations, and Hardening

## Scope

M8 makes Linkdrop deployment-ready on Ahara. It completes the Terraform root,
local deploy path, smoke checks, alarms, operator app authorization grant, and
operations documentation needed to run Linkdrop in production.

M8 does not add product API behavior, Android offline behavior, provider
credentials, provider embeds, new database tables, export flows, or any
per-project platform primitives that Ahara already provides. In particular, it
must not create a per-project VPC, ALB, RDS instance, API Gateway, Cognito user
pool, NAT gateway, Terraform state bucket, or standalone Cognito authorization
system.

Reference behavior comes from `LINKDROP-PLAN.md`,
`docs/adr/0001-ahara-platform-topology.md`,
`docs/adr/0002-capture-first-async-enrichment.md`,
`docs/adr/0003-project-owned-snapshot-storage.md`,
`../ahara/INTEGRATION.md`,
`../ahara-standards/standards/terraform.md`,
`../ahara-standards/standards/scripts.md`,
`../ahara-standards/standards/testing.md`,
`../ahara-tf-patterns/modules/platform-context`,
`../ahara-tf-patterns/modules/alb-api`,
`../ahara-tf-patterns/modules/lambda`,
`../ahara-tf-patterns/modules/website`,
`../ahara-tf-patterns/modules/cognito-app`,
`../tastebase/infrastructure/terraform`, and
`../ahara-business/infrastructure/terraform/app_authorizations.tf`:

- Linkdrop uses Ahara's shared ALB, VPC, PostgreSQL/RDS, Cognito pool, Route 53
  zone, and shared Terraform state.
- Terraform consumes Ahara resources through `platform-context` and the
  `alb-api`, `lambda`, `website`, and `cognito-app` modules.
- `GET /health` is public; all other API routes are behind shared ALB JWT
  validation.
- Web and Android use the Linkdrop public Cognito app client named
  `linkdrop-app`, and the platform auth trigger maps that client through
  `/ahara/auth-trigger/clients/linkdrop-app`.
- Backend runtime config comes from SSM, platform-context, module outputs, and
  Linkdrop-owned resource names; no secrets are written into repo files.
- Thumbnail snapshots are stored in a private Linkdrop-owned S3 bucket and read
  through the authenticated API.
- Capture remains write-first; `PROCESSING_FUNCTION_NAME` enables best-effort
  async enrichment dispatch, but dispatch failure must not block capture.
- `scripts/deploy.sh` is the parameterless local deploy entry point. CI does
  not call it.
- Commands that touch AWS, database credentials, Cognito, or live APIs are run
  with `with-cred --`.
- Rust and TypeScript cognitive complexity, function length, and file length
  gates remain enforced by `make ci`.

The M8 exit gate is `make ci` green plus a credentialed Terraform
format/validate/plan path against Ahara modules, with documented deploy and
smoke commands ready for operator execution.

## Steps

1. Register all deployable Lambda artifacts and align the local deploy script

   File(s): `platform.yml`, `scripts/deploy.sh`, `scripts/README.md`

   Reference behavior: the Ahara CI/deploy contract builds each registered Rust
   Lambda artifact explicitly. Linkdrop has both an HTTP API Lambda and an
   async processing Lambda. The local deploy script remains parameterless,
   builds release Lambda artifacts, builds the frontend, runs platform database
   migrations, applies Terraform with the shared state bucket, and prints useful
   outputs after apply.

   Change: add `processing` beside `api` in `platform.yml`. Replace the
   workspace debug build in `scripts/deploy.sh` with
   `cargo lambda build --release`, preserve the frontend build, `db-migrate`,
   and Terraform init/apply order, and print `frontend_url` and `api_url`
   outputs after apply. Update `scripts/README.md` to describe the Lambda build
   and output behavior without adding secrets or `.env` values.

   Verify: before the change, `processing`, the release Lambda build, and output
   printing are absent. After the change, run:

   ```sh
   rg "processing" platform.yml
   rg "cargo lambda build --release|output -raw frontend_url|output -raw api_url" scripts/deploy.sh
   bash -n scripts/deploy.sh
   ```

2. Add shared platform context and runtime configuration locals

   File(s): `infrastructure/terraform/context.tf`,
   `infrastructure/terraform/ssm.tf`,
   `infrastructure/terraform/locals.tf`

   Reference behavior: Linkdrop must read shared Ahara VPC, ALB, Cognito, RDS,
   and Route 53 context through `platform-context`, and per-project database
   credentials from `/ahara/db/linkdrop/*` SSM parameters. Runtime environment
   variables must match `backend/shared/src/config.rs` plus the already used
   snapshot and processing dispatch variables.

   Change: add a single `module "ctx"` using
   `git::https://github.com/chris-arsenault/ahara-tf-patterns.git//modules/platform-context`.
   Add data sources for `/ahara/db/linkdrop/username`,
   `/ahara/db/linkdrop/password`, and `/ahara/db/linkdrop/database`. Extend
   locals with `product_name`, `db_env`, `common_env`, and Lambda sizing or
   concurrency constants used by later Terraform steps. `db_env` must use
   `module.ctx.rds.address`, `module.ctx.rds.port`, and the Linkdrop DB SSM
   parameters. `common_env` must include `DB_HOST`, `DB_PORT`, `DB_NAME`,
   `DB_USERNAME`, `DB_PASSWORD`, `API_BASE_URL`, `APP_BASE_URL`,
   `COGNITO_USER_POOL_ID`, `COGNITO_CLIENT_ID`, `COGNITO_DOMAIN`, and
   `COGNITO_ISSUER`; the Cognito values come from `module.ctx.cognito` and
   `module.cognito_app.client_id`.

   Verify: before the change, the context module and Linkdrop SSM DB parameter
   reads are absent. After the change, run:

   ```sh
   rg 'module "ctx"|/ahara/db/linkdrop/(username|password|database)' infrastructure/terraform
   rg "common_env|COGNITO_CLIENT_ID|COGNITO_ISSUER|API_BASE_URL|APP_BASE_URL" infrastructure/terraform/locals.tf
   terraform fmt -check infrastructure/terraform/context.tf infrastructure/terraform/ssm.tf infrastructure/terraform/locals.tf
   ```

3. Create the private snapshot bucket resources

   File(s): `infrastructure/terraform/snapshot_storage.tf`,
   `infrastructure/terraform/outputs.tf`

   Reference behavior: snapshots are Linkdrop-owned archival copies, not source
   hotlinks. The bucket is private, blocks public access, uses server-side
   encryption, and is the only bucket referenced by the thumbnail archive and
   read paths.

   Change: add the `aws_s3_bucket` for `local.snapshot_bucket`, public access
   block, server-side encryption, ownership controls, and versioning enabled.
   Do not add expiration lifecycle rules because snapshots are the permanent
   archived copy. Add outputs for the snapshot bucket name and ARN. Do not
   create a public website bucket here; the frontend website module owns its own
   bucket.

   Verify: before the change, no snapshot bucket resource exists. After the
   change, run:

   ```sh
   rg 'resource "aws_s3_bucket" "snapshots"|aws_s3_bucket_public_access_block" "snapshots"|snapshot_bucket' infrastructure/terraform
   terraform fmt -check infrastructure/terraform/snapshot_storage.tf infrastructure/terraform/outputs.tf
   ```

4. Add the shared Cognito app client and auth-trigger client mapping [depends on #2]

   File(s): `infrastructure/terraform/cognito.tf`,
   `infrastructure/terraform/outputs.tf`

   Reference behavior: Linkdrop uses the shared Cognito pool with a public app
   client named `linkdrop-app`. The `cognito-app` module creates the app client,
   but the Ahara pre-auth trigger requires an SSM client mapping at
   `/ahara/auth-trigger/clients/linkdrop-app` whose value is the app client ID.

   Change: add `module "cognito_app"` from the `cognito-app` Ahara module with
   `name = "${local.prefix}-app"` and `cognito = module.ctx.cognito`. Add an
   `aws_ssm_parameter` for `/ahara/auth-trigger/clients/linkdrop-app` with the
   module's client ID. Add an output for the Linkdrop Cognito client ID.

   Verify: before the change, the Cognito module and auth-trigger SSM mapping
   are absent. After the change, run:

   ```sh
   rg 'module "cognito_app"|/ahara/auth-trigger/clients/linkdrop-app|module.cognito_app.client_id' infrastructure/terraform/cognito.tf infrastructure/terraform/outputs.tf
   terraform fmt -check infrastructure/terraform/cognito.tf infrastructure/terraform/outputs.tf
   ```

5. Wire the HTTP API and async processing Lambdas through Ahara modules [depends on #2] [depends on #3] [depends on #4]

   File(s): `infrastructure/terraform/lambdas.tf`,
   `infrastructure/terraform/outputs.tf`

   Reference behavior: the API runs behind the shared ALB through `alb-api`.
   `GET /health` is unauthenticated and all other API paths are authenticated by
   shared ALB JWT validation. The processing Lambda is not HTTP-triggered; it
   uses the standalone `lambda` module, shares the API role, and is invoked
   asynchronously by the API through `PROCESSING_FUNCTION_NAME`. Both Lambdas
   receive the Linkdrop runtime config and snapshot bucket name.

   Change: add `module "api"` from the `alb-api` module with prefix, API
   hostname, `module.ctx.vpc`, `module.ctx.alb`, `module.ctx.cognito`, and
   `environment = local.common_env`. Configure the API Lambda binary at
   `backend/target/lambda/api/bootstrap` with two route rules:
   `local.alb_priorities.api_health` for `GET`/`HEAD` `/health` with
   `authenticated = false`, and `local.alb_priorities.api_authenticated` for
   `/*` with authentication enabled. Add `PROCESSING_FUNCTION_NAME` and
   `SNAPSHOT_BUCKET` to the API Lambda environment. Add `module "processing"`
   from the standalone `lambda` module with binary
   `backend/target/lambda/processing/bootstrap`, `role_arn = module.api.role_arn`,
   `timeout = 300`, `module.ctx.vpc`, `local.common_env`, and
   `SNAPSHOT_BUCKET`. Add inline IAM policy statements for S3 read/write on the
   snapshot bucket and `lambda:InvokeFunction` on the processing Lambda. Add
   outputs for API and processing function names.

   Verify: before the change, neither module nor processing dispatch wiring
   exists in Terraform. After the change, run:

   ```sh
   rg 'module "api"|module "processing"|PROCESSING_FUNCTION_NAME|SNAPSHOT_BUCKET' infrastructure/terraform/lambdas.tf
   rg 'api_health|api_authenticated|authenticated = false|lambda:InvokeFunction|s3:GetObject|s3:PutObject' infrastructure/terraform/lambdas.tf
   terraform fmt -check infrastructure/terraform/lambdas.tf infrastructure/terraform/outputs.tf
   ```

6. Add the frontend website module and runtime config [depends on #4] [depends on #5]

   File(s): `infrastructure/terraform/frontend.tf`,
   `infrastructure/terraform/outputs.tf`

   Reference behavior: the React app is deployed through the Ahara `website`
   module, not a hand-built CloudFront/S3 stack. Runtime config is injected as
   `window.__APP_CONFIG__` and must satisfy `frontend/src/config.ts`:
   `apiBaseUrl`, `appBaseUrl`, `productName`, `cognitoUserPoolId`, and
   `cognitoClientId`.

   Change: add `module "frontend"` from the `website` module with prefix,
   frontend hostname, and `site_directory = "${path.module}/../../frontend/dist"`.
   Populate `runtime_config` with the Linkdrop API URL, app URL, product name,
   shared Cognito user pool ID, and Linkdrop app client ID. Add outputs for
   frontend URL, frontend bucket name, CloudFront distribution ID, and
   distribution domain name, reusing module outputs where available.

   Verify: before the change, the website module and runtime config do not
   exist. After the change, run:

   ```sh
   rg 'module "frontend"|runtime_config|apiBaseUrl|appBaseUrl|productName|cognitoUserPoolId|cognitoClientId' infrastructure/terraform/frontend.tf
   rg 'frontend_bucket|frontend_distribution' infrastructure/terraform/outputs.tf
   terraform fmt -check infrastructure/terraform/frontend.tf infrastructure/terraform/outputs.tf
   ```

7. Add production CloudWatch alarms for Linkdrop Lambdas [depends on #5]

   File(s): `infrastructure/terraform/alarms.tf`,
   `infrastructure/terraform/outputs.tf`

   Reference behavior: M8 requires operational alarms without creating a new
   notification system. Ahara publishes the shared alarm SNS topic in SSM, and
   Linkdrop should attach Lambda error and throttle alarms for the API and
   processing functions to that topic.

   Change: read `/ahara/alarms/sns-topic-arn` from SSM. Add CloudWatch metric
   alarms for `Errors` and `Throttles` on each deployed Lambda function name,
   with `alarm_actions` and `ok_actions` pointing to the shared topic. Add
   outputs for the alarm names. Keep thresholds simple and production-oriented:
   any error or throttle in a short evaluation window should alarm.

   Verify: before the change, no Linkdrop alarm resources exist. After the
   change, run:

   ```sh
   rg '/ahara/alarms/sns-topic-arn|aws_cloudwatch_metric_alarm" "lambda_errors|aws_cloudwatch_metric_alarm" "lambda_throttles' infrastructure/terraform/alarms.tf
   rg 'lambda_error_alarm_names|lambda_throttle_alarm_names' infrastructure/terraform/outputs.tf
   terraform fmt -check infrastructure/terraform/alarms.tf infrastructure/terraform/outputs.tf
   ```

8. Grant the operator account access through Ahara Business app authorizations

   File(s): `../ahara-business/infrastructure/terraform/app_authorizations.tf`

   Reference behavior: the shared Cognito pre-auth flow authorizes users through
   `ahara-business-app-authorizations`. M8 must verify the operator grant path
   by adding Linkdrop to the existing seeded operator account's app map instead
   of bypassing app authorization inside Linkdrop.

   Change: add `"linkdrop" = { S = "admin" }` to the `chris` seeded app
   authorization map. Do not add a Linkdrop-specific auth table, Cognito pool,
   or app authorization bypass.

   Verify: before the change, the seeded operator app map lacks Linkdrop. After
   the change, run:

   ```sh
   rg '"linkdrop"\s*=\s*\{ S = "admin" \}' ../ahara-business/infrastructure/terraform/app_authorizations.tf
   terraform -chdir=../ahara-business/infrastructure/terraform fmt -check app_authorizations.tf
   ```

9. Add the post-deploy smoke check script

   File(s): `scripts/smoke.sh`, `scripts/README.md`, `.env.example`

   Reference behavior: smoke checks exercise the deployed Ahara path without
   checking secrets into the repo. Public health can run unauthenticated.
   Authenticated checks require an operator-provided access token and call the
   same API routes used by web and Android. The script must be safe to run
   repeatedly and must not require a capture mutation unless the operator opts
   in.

   Change: add an executable `scripts/smoke.sh` that reads
   `API_BASE_URL` with a default of `https://api.linkdrop.ahara.io`, checks
   `/health`, and, when `LINKDROP_ACCESS_TOKEN` is present, checks `/me`,
   `/items`, and `/tags`. Add optional capture smoke behavior gated by
   `LINKDROP_SMOKE_CAPTURE_URL`; it should POST a URL with no mandatory tags
   and confirm the response contains an item/copy URL without depending on
   external provider enrichment. Document `with-cred -- scripts/smoke.sh` and
   placeholder env names only.

   Verify: before the change, the smoke script and token/capture smoke contract
   are absent. After the change, run:

   ```sh
   test -x scripts/smoke.sh
   bash -n scripts/smoke.sh
   rg 'LINKDROP_ACCESS_TOKEN|LINKDROP_SMOKE_CAPTURE_URL|/health|/me|/items|/tags' scripts/smoke.sh scripts/README.md .env.example
   ```

10. Document deployment, runtime resources, and operations [depends on #1] [depends on #3] [depends on #7] [depends on #9]

   File(s): `README.md`, `docs/development.md`, `docs/architecture.md`,
   `docs/backlog.md`, `infrastructure/terraform/README.md`,
   `backend/README.md`, `backend/processing/README.md`, `CHANGELOG.md`

   Reference behavior: docs should reflect the actual M8 deploy surface without
   claiming a production apply has already happened. They must point operators
   to the shared Ahara deploy contract, parameterless local deploy script,
   credentialed Terraform plan path, smoke checks, outputs, alarms, snapshot
   bucket, and auth-trigger client mapping.

   Change: update documentation to describe the completed Terraform resources,
   runtime configuration, deploy and smoke commands, alarm coverage, and
   operator app authorization grant. Remove stale statements that Terraform
   resource wiring, snapshot bucket creation, alarms, or deploy/smoke paths are
   future work. Keep out-of-scope provider credentials, provider embeds,
   Android offline queueing, and production apply evidence in backlog/future
   sections rather than presenting them as complete.

   Verify: before the change, docs still describe deployment wiring as future
   or incomplete. After the change, run:

   ```sh
   rg 'cargo lambda build|PROCESSING_FUNCTION_NAME|snapshot bucket|scripts/smoke.sh|terraform plan|with-cred|auth-trigger|CloudWatch' README.md docs infrastructure/terraform backend CHANGELOG.md
   rg 'provider credentials|provider embeds|Android offline|production apply' docs/backlog.md README.md docs || true
   ```

11. Validate Terraform shape and plan against Ahara modules [depends on #1] [depends on #2] [depends on #3] [depends on #4] [depends on #5] [depends on #6] [depends on #7] [depends on #8]

   File(s): `infrastructure/terraform/*.tf`, `scripts/deploy.sh`,
   `platform.yml`, `../ahara-business/infrastructure/terraform/app_authorizations.tf`

   Reference behavior: M8 must exercise the deploy flow without creating
   project-owned platform primitives. Terraform should initialize, validate,
   and produce a plan against Ahara modules and shared data sources. Live AWS,
   Cognito, SSM, and database credential access must be wrapped with
   `with-cred --`.

   Change: make only syntax, wiring, output, or permission adjustments required
   for Terraform format, validate, and plan to succeed. Do not add new product
   behavior or refactor code outside the named files. Do not commit `.terraform`
   directories or plan files.

   Verify: before the Terraform wiring exists, init/validate/plan cannot
   succeed for the production graph and the forbidden-resource check has little
   to inspect. After the change, run:

   ```sh
   terraform fmt -check -recursive infrastructure/terraform
   with-cred -- terraform -chdir=infrastructure/terraform init -backend=false
   with-cred -- terraform -chdir=infrastructure/terraform validate
   with-cred -- terraform -chdir=infrastructure/terraform plan -refresh=false -input=false -out=/tmp/linkdrop-m8.tfplan
   if rg 'resource "aws_(vpc|lb|db_instance|api_gateway|apigatewayv2_api|cognito_user_pool|nat_gateway)"' infrastructure/terraform; then exit 1; fi
   ```

## Exit Gate

Run the phase exit gate after all steps above are complete:

```sh
make ci
terraform fmt -check -recursive infrastructure/terraform
with-cred -- terraform -chdir=infrastructure/terraform init -backend=false
with-cred -- terraform -chdir=infrastructure/terraform validate
with-cred -- terraform -chdir=infrastructure/terraform plan -refresh=false -input=false -out=/tmp/linkdrop-m8.tfplan
if rg 'resource "aws_(vpc|lb|db_instance|api_gateway|apigatewayv2_api|cognito_user_pool|nat_gateway)"' infrastructure/terraform; then exit 1; fi
```

M8 is complete only when those commands pass and the resulting plan is ready
for an operator to execute with the documented `with-cred -- scripts/deploy.sh`
and `with-cred -- scripts/smoke.sh` paths.
