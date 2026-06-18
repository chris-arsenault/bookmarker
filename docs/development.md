# Development

## Standards

Linkdrop follows the Ahara project standards:

| Area       | Standard                                                                                    |
| ---------- | ------------------------------------------------------------------------------------------- |
| Platform   | [../ahara/INTEGRATION.md](../../ahara/INTEGRATION.md)                                       |
| TypeScript | [../ahara-standards/standards/typescript.md](../../ahara-standards/standards/typescript.md) |
| Rust       | [../ahara-standards/standards/rust.md](../../ahara-standards/standards/rust.md)             |
| Terraform  | [../ahara-standards/standards/terraform.md](../../ahara-standards/standards/terraform.md)   |
| Testing    | [../ahara-standards/standards/testing.md](../../ahara-standards/standards/testing.md)       |

## Commands

| Command                                         | Purpose                                                                            |
| ----------------------------------------------- | ---------------------------------------------------------------------------------- |
| `make ci`                                       | Canonical local verification target                                                |
| `make db-test`                                  | Docker-backed PostgreSQL migration/repository/processing integration tests         |
| `make build`                                    | Build the registered Rust workspace, frontend shell, and Android debug APK         |
| `cd frontend && pnpm run desktop:build`         | Build the Electron desktop main/preload entrypoints                                |
| `cd frontend && pnpm run desktop:start`         | Build frontend + Electron entrypoints and start the desktop shell                  |
| `make desktop-package`                          | Build a runnable current-platform Electron shell under `frontend/release/`         |
| `make android-release-build`                    | Build the Android release APK                                                      |
| `make android-create-release-keystore`          | Create the local Android release keystore without overwriting an existing one      |
| `make android-sign-release`                     | Build, align, sign, and verify the Android release APK                             |
| `make android-install-debug`                    | Build and install the debug APK on one attached Android device                     |
| `make android-install-release`                  | Sign if needed, then install the signed release APK on one attached Android device |
| `make deploy`                                   | Run the local deploy script                                                        |
| `cd frontend && pnpm install --frozen-lockfile` | Install frontend dependencies from the lockfile                                    |

`make ci` checks Rust lint/unit tests/API route tests, frontend
lint/typecheck/tests, Electron desktop typecheck, documentation scaffold,
Terraform formatting, Android share target structure, and Android
`:app:assembleDebug`. It does not run the Docker-backed PostgreSQL integration
suite; use `make db-test` before database migration, PostgreSQL repository, or
processing queue changes. `make ci` enforces Rust Clippy warnings, Rust
cognitive complexity `10`, Rust function length `75`, Rust source files under
`400` lines, TypeScript/React cognitive complexity `10`, TypeScript files under
`400` lines, and TypeScript functions under `75` lines. Android builds use the
checked-in Gradle wrapper and require `ANDROID_HOME` or `ANDROID_SDK_ROOT` to
point at an SDK with platform `android-36`; `$HOME/android-sdk` is used when
neither variable is set. Android APK outputs are named for the product and
variant, such as `linkdrop-debug-v0.1.0-1.apk` and
`linkdrop-release-unsigned-v0.1.0-1.apk`. The release APK is unsigned until a
real release signing config is provided.

## Deploy and smoke

Use the secret broker for commands that touch AWS, Cognito, database
credentials, or live API tokens:

```bash
with-cred -- terraform -chdir=infrastructure/terraform plan -refresh=false -input=false -out=/tmp/linkdrop-m8.tfplan
with-cred -- scripts/deploy.sh
with-cred -- scripts/smoke.sh
```

`scripts/deploy.sh` is parameterless. It builds Rust Lambda release artifacts
with `cargo lambda build --release`, builds the frontend, runs `db-migrate`,
applies Terraform against the shared Ahara state bucket, and prints the
`frontend_url` and `api_url` outputs. `scripts/smoke.sh` checks `/health` by
default and adds authenticated `/me`, `/items`, `/tags`, and optional zero-tag
capture checks when `LINKDROP_ACCESS_TOKEN` and `LINKDROP_SMOKE_CAPTURE_URL`
are present.

## Local servers

Start local development servers only when the user explicitly requests one.
