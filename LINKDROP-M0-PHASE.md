# M0 — Platform-Ready Scaffold Phase Plan

This expands only `M0 — Platform-Ready Scaffold` from [LINKDROP-PLAN.md](LINKDROP-PLAN.md). The feature-start pass already created the documentation surface and placeholder component homes; this phase completes the buildable/deployable scaffold without implementing capture, library, enrichment, database schema, web workflows, or Android share behavior.

## Reference Context

- Plan context/reuse map: [LINKDROP-PLAN.md](LINKDROP-PLAN.md)
- Platform topology decision: [docs/adr/0001-ahara-platform-topology.md](docs/adr/0001-ahara-platform-topology.md)
- Ahara integration contract: `../ahara/INTEGRATION.md`
- Shared CI contract: `../ahara/CI-WORKFLOW.md`
- Standards: `../ahara-standards/standards/project-structure.md`, `scripts.md`, `typescript.md`, `rust.md`, `terraform.md`, `testing.md`, `git.md`
- Existing platform registration patterns: `../ahara-infra/infrastructure/terraform/control/project-tastebase.tf`, `../ahara-infra/infrastructure/terraform/control/project-ahara-business.tf`, `../ahara-infra/infrastructure/terraform/services/db-migrate.tf`

## Steps

1. Add root runtime pins and local environment template.
   - File(s): `.node-version`, `.prettierrc`, `.env.example`, `.gitignore`
   - Reference behavior: Ahara scripts and TypeScript standards require exact Node pinning, exact pnpm pinning in the frontend package, committed env examples without real secrets, and standard Prettier formatting. `.gitignore` must keep local secrets and build artifacts out of git.
   - Change: add `.node-version` with `24.12.0`; add the standard Prettier config; add a root `.env.example` with placeholder values for state bucket/region, app/API URLs, Cognito, database, and snapshot storage; keep `.gitignore` aligned with Ahara git standards.
   - Verify: `test "$(cat .node-version)" = "24.12.0" && test -f .env.example && node -e 'JSON.parse(require("fs").readFileSync(".prettierrc", "utf8"))'`. Red before: `.node-version`, `.env.example`, and `.prettierrc` do not exist.

2. Create the minimal Rust workspace shell.
   - File(s): `backend/Cargo.toml`, `backend/clippy.toml`, `backend/rustfmt.toml`, `backend/shared/Cargo.toml`, `backend/shared/src/lib.rs`, `backend/api/README.md`, `backend/processing/README.md`, `backend/README.md`
   - Reference behavior: Ahara Rust standards use a workspace, copied clippy/rustfmt settings from `ahara-standards`, strict clippy, and `lib.rs` for testable logic. `M0` must avoid half-registered build members, so `api` and `processing` remain reserved homes until their phases add buildable crates.
   - Change: register only a minimal `shared` crate in the workspace; expose a small service-name/config constant with a unit test; copy the clippy and rustfmt settings from `../ahara-standards/rules/rust/`; add README notes for future `api` and `processing` crates without adding them to `members`.
   - Verify: `cd backend && cargo test --workspace --lib && cargo fmt -- --check && cargo clippy --workspace --all-targets -- -D warnings -W clippy::cognitive_complexity`. Red before: `backend/Cargo.toml` and the `shared` crate do not exist.

3. Create the minimal frontend shell.
   - File(s): `frontend/package.json`, `frontend/pnpm-lock.yaml`, `frontend/index.html`, `frontend/vite.config.ts`, `frontend/tsconfig.json`, `frontend/tsconfig.app.json`, `frontend/tsconfig.node.json`, `frontend/eslint.config.js`, `frontend/src/main.tsx`, `frontend/src/App.tsx`, `frontend/src/App.test.tsx`, `frontend/src/config.ts`, `frontend/src/index.css`, `frontend/src/vite-env.d.ts`, `frontend/README.md`
   - Reference behavior: Ahara TypeScript standards require pnpm, `packageManager: "pnpm@10.29.3"`, Vite/React/TypeScript, ESLint flat config with `@ahara/standards` rules, Vitest, no raw direct fetch in views, and runtime config via `window.__APP_CONFIG__`.
   - Change: add a minimal React app shell and config reader with no API calls; add the standard lint/test/build scripts and dependency cohort from current Ahara React projects; generate the pnpm lockfile from the declared package.
   - Verify: `cd frontend && pnpm install --frozen-lockfile && pnpm exec eslint . && pnpm exec tsc --noEmit && pnpm exec vitest run`. Red before: `frontend/package.json` and `frontend/src/` do not exist.

4. Add the Terraform root and reserve Linkdrop listener priorities.
   - File(s): `infrastructure/terraform/main.tf`, `infrastructure/terraform/locals.tf`, `infrastructure/terraform/outputs.tf`, `infrastructure/terraform/README.md`
   - Reference behavior: Ahara Terraform standards require Terraform `>= 1.12`, AWS provider `~> 6.0`, backend key `projects/linkdrop.tfstate`, default tags, `platform-context` for shared discovery when resources are added, and no `terraform_remote_state`. Current consumer priority ranges leave `380-389` unused.
   - Change: add a no-resource Terraform root with backend/provider configuration, Linkdrop locals for prefix/hosts/snapshot bucket naming, and reserved ALB listener priority locals such as `api_health = 380` and `api_authenticated = 381`. Keep the root deployable without creating AWS resources in M0.
   - Verify: `terraform fmt -check -recursive infrastructure/terraform/ && rg "api_health\\s*=\\s*380|api_authenticated\\s*=\\s*381" infrastructure/terraform/locals.tf`. Red before: Terraform root files and priority locals do not exist.

5. Add the platform manifest and shared CI workflow.
   - File(s): `platform.yml`, `.github/workflows/ci.yml`, `Makefile`
   - Reference behavior: Ahara CI reads `platform.yml`; standard projects use `chris-arsenault/ahara/.github/workflows/ci.yml@main`; the Makefile `ci` target mirrors the declared stack. M0 has Rust code but no deployable Lambda artifacts yet, so `rust_artifacts: {}` is the correct non-deployable Rust declaration.
   - Change: add `platform.yml` with project/prefix `linkdrop`, stack `rust`, `typescript`, `terraform`, migrations at `db/migrations`, and `rust_artifacts: {}`; add the shared reusable CI workflow with OIDC/content permissions; update `Makefile` with `lint`, `fmt`, `typecheck`, `test`, `terraform-fmt-check`, `docs-check`, `build`, and `deploy` targets using the M0 scaffolds.
   - Verify: `test -f platform.yml && test -f .github/workflows/ci.yml && rg "rust_artifacts: \\{\\}" platform.yml && make ci`. Red before: `platform.yml` and `.github/workflows/ci.yml` do not exist, and the current `make ci` does not exercise Rust/TypeScript/Terraform checks.

6. Add the parameterless deploy script shell.
   - File(s): `scripts/deploy.sh`, `scripts/README.md`, `Makefile`
   - Reference behavior: Ahara deploy scripts live at `scripts/deploy.sh`, are parameterless, do not source `.env`, use state bucket/region defaults, run project build steps, run `db-migrate`, and apply Terraform locally. CI replicates deploy steps instead of calling this script.
   - Change: add an executable deploy script with the standard root/terraform directory discovery and state defaults; for M0, run backend/frontend build commands that exist, then `db-migrate`, then Terraform init/apply. Wire `make deploy` to the script.
   - Verify: `bash -n scripts/deploy.sh && test -x scripts/deploy.sh && rg "db-migrate" scripts/deploy.sh && rg "terraform -chdir" scripts/deploy.sh`. Red before: `scripts/deploy.sh` does not exist.

7. Add the Android project shell without share behavior.
   - File(s): `android/settings.gradle.kts`, `android/build.gradle.kts`, `android/app/build.gradle.kts`, `android/app/src/main/AndroidManifest.xml`, `android/app/src/main/java/io/ahara/linkdrop/MainActivity.kt`, `android/README.md`
   - Reference behavior: The confirmed architecture puts the native Android client under `android/`, but Ahara has no Android CI or deployment standard. M3 owns `ACTION_SEND` share-target behavior, so M0 only reserves a conventional native Android home.
   - Change: add a minimal Kotlin Android project shell, package namespace `io.ahara.linkdrop`, and a placeholder `MainActivity`. Do not add share intent filters or API/auth behavior in M0.
   - Verify: `test -f android/app/src/main/AndroidManifest.xml && rg "namespace = \"io\\.ahara\\.linkdrop\"" android/app/build.gradle.kts && ! rg "ACTION_SEND" android/app/src/main/AndroidManifest.xml`. Red before: Android Gradle and manifest files do not exist. Android compile verification is intentionally omitted in M0 because Java/Gradle are not installed in this environment and no Ahara Android CI contract exists yet.

8. Register the Linkdrop deployer in `ahara-infra`.
   - File(s): `../ahara-infra/infrastructure/terraform/control/project-linkdrop.tf`
   - Reference behavior: `../ahara/INTEGRATION.md` requires cross-repo deployer registration. Current `ahara-infra` project files use `local.github_pat`, `allowed_repos` as the GitHub repo name, `prefix` for AWS scope, module bundles for shared modules, and `ssm_additional_parameter_paths` when a project writes an auth-trigger client mapping.
   - Change: add `module "project_linkdrop"` with `allowed_repos = ["bookmarker"]`, `prefix = "linkdrop"`, `state_key_prefix = "projects/linkdrop"`, module bundles `website`, `alb-api`, `cognito-app`, and `lambda`, `ssm_additional_parameter_paths = ["ahara/auth-trigger/clients/linkdrop-app"]`, and policy modules for `terraform-state`, `db-migrate`, `s3-private-storage`, `ssm-write`, and `cloudwatch-alarms`.
   - Verify: `terraform -chdir=../ahara-infra/infrastructure/terraform fmt -check control/project-linkdrop.tf && rg "module \"project_linkdrop\"|allowed_repos\\s*=\\s*\\[\"bookmarker\"\\]|auth-trigger/clients/linkdrop-app" ../ahara-infra/infrastructure/terraform/control/project-linkdrop.tf`. Red before: `project-linkdrop.tf` does not exist.

9. Register the Linkdrop database in `ahara-infra`.
   - File(s): `../ahara-infra/infrastructure/terraform/services/db-migrate.tf`
   - Reference behavior: Ahara database projects are registered in `var.migration_projects`; the migration Lambda creates the database, app role, grants, and SSM credentials. Project migrations must not create roles, users, grants, or databases.
   - Change: add `linkdrop = { db_name = "linkdrop" }` to `migration_projects`.
   - Verify: `terraform -chdir=../ahara-infra/infrastructure/terraform fmt -check services/db-migrate.tf && rg "linkdrop\\s*=\\s*\\{ db_name = \"linkdrop\" \\}" ../ahara-infra/infrastructure/terraform/services/db-migrate.tf`. Red before: no `linkdrop` entry exists.

10. Tighten documentation to describe the completed M0 scaffold.
   - File(s): `README.md`, `AGENTS.md`, `docs/development.md`, `backend/README.md`, `frontend/README.md`, `android/README.md`, `infrastructure/terraform/README.md`, `scripts/README.md`
   - Reference behavior: Repo docs are current-state indexes; implementation history and trade-offs stay out of top-level files. The M0 scaffold should tell future executors which homes are build-registered and which homes are reserved.
   - Change: update the existing docs to reflect the actual M0 scaffold: `shared` is the only registered backend crate, `api`/`processing` are reserved, Android is a non-CI shell until M3, Terraform creates no Linkdrop AWS resources in M0, and `make ci` is the canonical gate.
   - Verify: `make ci && ! rg "we used to|previously|this was migrated from|we plan to|not yet|TODO|FIXME" README.md AGENTS.md CLAUDE.md docs backend frontend android infrastructure scripts`. Red before: docs still describe only reserved homes and do not capture the completed scaffold boundaries.

## Exit Gate

Run these after all steps:

```bash
make ci
terraform -chdir=../ahara-infra/infrastructure/terraform fmt -check control/project-linkdrop.tf services/db-migrate.tf
git status --short
git -C ../ahara-infra status --short
```

The phase is complete when `make ci` is green, the two `ahara-infra` Terraform files are formatted, Linkdrop has standard Ahara scaffolding, and no future build member is registered before it compiles.
