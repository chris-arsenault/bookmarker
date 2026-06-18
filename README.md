# Bookmarker

Bookmarker is a personal capture vault for links and short-lived text snippets.
Android share sheets, the web app, and the desktop clipboard shell all write to
the same Ahara-backed item library.

## Quickstart

```bash
cd frontend
pnpm install --frozen-lockfile
cd ..
make ci
```

`make ci` also compiles the Android debug APK through `android/gradlew`. Set
`ANDROID_HOME` or `ANDROID_SDK_ROOT` to an SDK with platform `android-36`, or
install that SDK under `$HOME/android-sdk`.
Docker-backed PostgreSQL integration tests are intentionally outside the default
local gate; run `make db-test` when changing migrations, PostgreSQL repository
code, or processing queue behavior.
Use `make android-release-build` to produce the release build artifact at
`android/app/build/outputs/apk/release/linkdrop-release-unsigned-v0.1.0-1.apk`. The
release APK is unsigned until a real release signing config is provided. Use
`make android-create-release-keystore`, `make android-sign-release`,
`make android-install-debug`, and `make android-install-release` for guarded
phone install/signing flows.

Use `make desktop-package` to build a runnable Electron shell from the installed
Electron runtime. On Windows this writes
`frontend/release/bookmarker-win32-x64/Bookmarker.exe`; on Linux this writes
`frontend/release/bookmarker-linux-x64/Bookmarker`. The packaged shell loads the
deployed Bookmarker web app by default, and `BOOKMARKER_DESKTOP_URL` can
override that URL.

Implementation is driven from [LINKDROP-PLAN.md](LINKDROP-PLAN.md). Expand one milestone at a time with the `plan-phase` workflow before writing production code.

## Architecture

| Layer          | Current state                                                                                                                                                                                                                                       |
| -------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Backend        | Rust workspace with authenticated API capture, async processing dispatch, metadata enrichment, filtered item reads, item organization mutations, tag cleanup routes, API-mediated thumbnail reads, tag corpus reads, and shared PostgreSQL services |
| Frontend       | Cognito-authenticated Vite React/TypeScript web vault for browsing links and text snippets, filtering, organization, source opening, canonical/text copy, and snapshot thumbnail display                                                            |
| Desktop        | Electron shell for explicit clipboard capture, tray access, and a compact always-on-top HUD backed by the same authenticated web app                                                                                                                |
| Android        | Native Kotlin `ACTION_SEND` share target with authenticated quick-drop URL or text capture and optional explicit tags                                                                                                                               |
| Database       | Platform migration directory with shared items, URL/text payloads, tags, snapshots, processing, and capture idempotency schema                                                                                                                      |
| Infrastructure | Terraform root using Ahara `platform-context`, `alb-api`, `lambda`, `website`, and `cognito-app` modules for the deployed app, API, processor, snapshot bucket, alarms, and runtime config                                                          |

## Capture

Authenticated clients call `POST /items` with a URL, optional explicit tags, and
an optional `client_capture_id`, or `POST /items/text` with snippet text and the
same optional tag/idempotency fields. Capture is immediate with no required
metadata fields. Reusing the same `client_capture_id` returns the existing item
for retry-safe share attempts.

Capture keeps the original submitted URL and stores a canonical URL when
normalization succeeds. The canonical URL strips common tracking parameters,
normalizes known short-share hosts such as `youtu.be`, resolves TikTok short
links such as `vt.tiktok.com` on a best-effort basis, and powers per-user
deduplication. URL item responses include `url.copy_url`, which is the canonical
URL when available and the original URL when normalization failed. Text snippet
responses include `text.plain_text` and `archive_status = not_applicable`.

After capture commits, the API queues the processing Lambda best effort. Capture
responses do not wait for metadata fetches or thumbnail downloads. Processing
updates `metadata_snapshots` with title, author/channel, platform, optional
duration, `archive_status`, and a Linkdrop-owned `thumbnail_s3_key` when a
thumbnail snapshot succeeds. Failed enrichment leaves the saved item visible
with `archive_status = failed`. New captures land with `inbox_status =
unsorted`; moving an item to `organized` is a later user action.

## Web Library

The web UI authenticates with the shared Cognito app client and reads the same
library as Android. It shows saved items in a visual feed/detail workspace with
title, platform, tags, date added, watched status, archive status, notes, and a
thumbnail when a Linkdrop-owned snapshot exists. Thumbnail images are loaded
through the authenticated API route rather than hotlinked from the source site.

The item list supports filters for platform, explicit tag, added date range,
`archive_status`, watched status, `inbox_status`, and free-text search over
title/notes. The detail panel supports notes editing, explicit tag replacement,
watched/unwatched changes, and unsorted/organized inbox changes. Tag management
supports tag rename and tag merge for typo cleanup while preserving item
associations and usage counts. Tags still come only from explicit user input.
Item actions open the live source URL for URL items, copy `url.copy_url` for URL
items, and copy `text.plain_text` for text snippets.

## URLs

| Surface | URL                             |
| ------- | ------------------------------- |
| App     | `https://linkdrop.ahara.io`     |
| API     | `https://api.linkdrop.ahara.io` |

## Deploy

```bash
with-cred -- terraform -chdir=infrastructure/terraform plan -refresh=false -input=false -out=/tmp/linkdrop-m8.tfplan
with-cred -- scripts/deploy.sh
with-cred -- scripts/smoke.sh
```

The deploy path builds Lambda release artifacts with `cargo lambda build --release`,
builds the frontend, runs platform database migrations, applies Terraform with
the shared Ahara state bucket, and prints the deployed app/API URLs. The smoke
script checks `/health`; with `LINKDROP_ACCESS_TOKEN` it also checks `/me`,
`/items`, and `/tags`; with `LINKDROP_SMOKE_CAPTURE_URL` it performs an
optional zero-tag capture smoke.

Terraform creates only Linkdrop-owned resources on shared Ahara infrastructure:
the ALB API Lambda, async processing Lambda, website, public Cognito app client,
auth-trigger SSM registration, private snapshot bucket, runtime config, and
CloudWatch Lambda alarms. It does not create a per-project VPC, ALB, RDS,
API Gateway, Cognito user pool, NAT gateway, or state bucket. Operator access
is granted through the shared `ahara-business-app-authorizations` path.

## Documentation

| Topic                  | Link                                         |
| ---------------------- | -------------------------------------------- |
| Implementation plan    | [LINKDROP-PLAN.md](LINKDROP-PLAN.md)         |
| Architecture           | [docs/architecture.md](docs/architecture.md) |
| Development            | [docs/development.md](docs/development.md)   |
| Architecture decisions | [docs/adr/README.md](docs/adr/README.md)     |
| Backlog                | [docs/backlog.md](docs/backlog.md)           |
| Changelog              | [CHANGELOG.md](CHANGELOG.md)                 |
| Agent guide            | [AGENTS.md](AGENTS.md)                       |

## License

MIT. See [LICENSE](LICENSE).
