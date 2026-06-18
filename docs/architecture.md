# Architecture

Linkdrop is organized as a standard Ahara product repo with a native Android client.

## Components

| Component | Responsibility |
| ---- | ---- |
| `android/` | Native Android `ACTION_SEND` client for authenticated quick-drop capture. |
| `backend/api` | Authenticated Rust HTTP API Lambda behind the shared Ahara ALB. |
| `backend/processing` | Async metadata enrichment Lambda and thumbnail snapshot storage boundary. |
| `backend/shared` | Shared crate for platform identity, runtime config, Cognito auth, public errors, database helpers, M1 migration constants, domain types, and library services. |
| `frontend/` | Cognito-authenticated Vite React/TypeScript web vault plus Electron desktop shell entrypoints for private browsing, filtering, organization, source opening, copy actions, clipboard capture, HUD display, and snapshot thumbnails. |
| `db/migrations/` | Defines the PostgreSQL model for items, URLs, explicit tags, tag usage, notes, archive snapshots, and processing state. |
| `infrastructure/terraform/` | Project Terraform root for Ahara platform resources. |

## Platform integration

The API runs as a Rust Lambda behind the shared Ahara ALB through the
`alb-api` module. `GET /health` is public; all other API routes are protected by
the shared ALB JWT validation action. The async enrichment worker runs as a
standalone Lambda through the Ahara `lambda` module, and the API invokes it
best effort through `PROCESSING_FUNCTION_NAME`.

Terraform uses the shared `platform-context`, `alb-api`, `lambda`, `website`,
and `cognito-app` modules from `ahara-tf-patterns`. It reads shared VPC, ALB,
Cognito, Route 53, and RDS context rather than creating per-project platform
infrastructure. The backend uses the shared PostgreSQL/RDS instance with a
per-project database named `linkdrop`. Web and Android clients use the shared
Cognito pool through the Linkdrop app client. The app client is registered with
the platform auth trigger through `/ahara/auth-trigger/clients/linkdrop-app`.

## Current API surface

M7 exposes authenticated quick-drop capture, filtered item/tag reads,
organization mutations, tag cleanup, API-mediated thumbnail snapshot reads, and
capture-time dispatch into asynchronous processing:

| Route | Auth | Contract |
| ---- | ---- | ---- |
| `GET /health` | No | Return service health. |
| `GET /me` | Yes | Return the authenticated Cognito user context. |
| `POST /items` | Yes | Capture a URL with optional explicit tags and optional `client_capture_id`. |
| `POST /items/text` | Yes | Capture a text snippet with optional explicit tags, source metadata, and optional `client_capture_id`. |
| `GET /items` | Yes | Return item summaries scoped to the current user, with optional platform, tag, date, archive status, watched status, inbox status, and text filters. |
| `GET /items/{item_id}` | Yes | Return item detail scoped to the current user. |
| `GET /items/{item_id}/thumbnail` | Yes | Return Linkdrop-owned thumbnail snapshot bytes for an owned item. |
| `GET /tags` | Yes | Return the current user's explicit tag corpus. |
| `PATCH /items/{item_id}` | Yes | Update notes, explicit tags, `watch_status`, and `inbox_status` for an existing item. |
| `PATCH /tags/{tag_id}` | Yes | Rename a tag when the normalized destination does not collide. |
| `POST /tags/{source_tag_id}/merge` | Yes | Merge a source tag into a different target tag. |

Every authenticated route uses the shared Cognito verifier. Public API errors
use a stable JSON shape with a safe `code` and `message`.

## Capture model

Capture is write-first. `POST /items` persists a quick-drop record as soon as it
receives a URL and optional explicit tags. Android supplies a stable
`client_capture_id` per share attempt so user retries return the existing item
instead of creating another pending item.

M4 normalizes URLs during capture for storage, deduplication, and copy behavior.
Tracking parameters are stripped, `youtu.be` is converted without network
access, and TikTok short links such as `vt.tiktok.com` are resolved through a
bounded best-effort resolver. If short-link resolution fails, capture still saves
the item with the original URL and a `copy_url` fallback.

M5 processing updates the same record after metadata enrichment and snapshot
archival. The API queues processing best effort after capture commits; dispatch
failure never changes the capture response.

The tag corpus is derived from explicit item-tag associations. Chip ranking uses
usage counts from that corpus and starts empty on a new install. New captures
land with `inbox_status = unsorted`; `organized` is set later by deliberate
user action.

## Database and domain model

M1 defines `users`, `items`, `item_urls`, `tags`, `item_tags`,
`tag_usage_counts`, `item_notes`, `metadata_snapshots`, and `processing_jobs`
in PostgreSQL. Rollback SQL removes those project-owned objects in reverse
dependency order.

M3 adds `items.client_capture_id` with per-user uniqueness for retry-safe
capture attempts.

M7 adds `items.inbox_status`, defaulting to `unsorted` with an `organized`
state for deliberate filing. The shared Rust domain exposes `SubmittedUrl`,
`TagName`, `ItemKind`, `TextSnippetBody`, `ArchiveStatus`, `WatchStatus`,
`InboxStatus`, `ProcessingJobKind`, and `ProcessingStatus`. The vault extension
adds `items.item_kind` and `item_texts` so URL captures and text snippets share
item organization while keeping payload-specific storage. `SubmittedUrl` validates
absolute HTTP(S) URLs without stripping query parameters or resolving
shorteners. `TextSnippetBody` rejects blank snippets while preserving submitted
text. `TagName` trims explicit user-entered text and derives the corpus key from
that text only.

## URL and archive model

Each item stores the original submitted URL and, when normalization succeeds, a
canonical URL. Deduplication uses the normalized canonical URL, so repeated
captures of equivalent links surface the existing item instead of creating
duplicates. API item summaries and detail responses include `copy_url`, which is
the canonical URL when present and the original URL when normalization is failed
or pending.

Thumbnail, title, author/channel, platform, duration, and archive status are
stored as snapshot fields in `metadata_snapshots`. The processing Lambda fetches
metadata best effort, writes `archive_status` as `pending`, `succeeded`, or
`failed`, and leaves failed sources saved and queryable. When a thumbnail is
available, it is downloaded and stored through a Linkdrop-owned snapshot store;
the database keeps `thumbnail_s3_key` and content type, not the source thumbnail
hotlink. Web clients read thumbnails through the authenticated API, which checks
item ownership before loading the stored snapshot object.

## Web UI

The web UI is a Cognito-authenticated vault workspace. It renders URL items and
text snippets in one feed/detail panel with title/preview, platform/source app,
explicit tags, date added, `archive_status`, watched status, `inbox_status`,
notes, and API-mediated thumbnail snapshots when available. It supports filters
for platform, explicit tag, added date range, archive status, watched status,
inbox status, and free-text title/snippet/notes search.

Item actions open the source link for URL items, copy `url.copy_url` for URL
items, and copy `text.plain_text` for text snippets. M6 uses source deep-links
instead of provider embeds because the current URL backend contract exposes
canonical/live source URLs, not provider embed URLs.

The Electron desktop shell loads the built frontend, exposes clipboard read and
write IPC through a preload bridge, adds a tray entry for explicit clipboard
capture, and provides a compact always-on-top HUD for copying recent unsorted
items without global keybinding overload.

M7 organization happens after capture in the detail panel. Users can edit notes,
replace explicit item tags, toggle watched/unwatched, and move items between
unsorted and organized. The tag management panel supports tag rename and tag
merge for explicit corpus cleanup; merge moves source associations to the
target and relies on database usage counts for chip ranking.

## Android UI

The Android app registers an `ACTION_SEND` share destination for text payloads.
It extracts a shared URL when present; otherwise non-empty shared text is saved
as a text snippet. It allows immediate save with no required fields, loads
optional explicit tag chips from the user's corpus, accepts one free-text tag,
and shows a non-blocking confirmation toast. The app signs in against the
shared Ahara Cognito pool through the Linkdrop public app client, stores tokens
locally, refreshes access tokens before API calls, and supports the platform
software-token `SOFTWARE_TOKEN_MFA` and `MFA_SETUP` challenge states.

## Operations

The deployment root creates the website, API Lambda, processing Lambda,
Linkdrop public Cognito client, auth-trigger SSM mapping, private snapshot
bucket, runtime config, and CloudWatch `Errors`/`Throttles` alarms for both
Lambdas. The snapshot bucket is private, blocks public access, uses server-side
encryption, and keeps versioning enabled because thumbnails are the archived
copy.

Operators use `with-cred -- scripts/deploy.sh` to build, migrate, apply
Terraform, and print outputs. `with-cred -- scripts/smoke.sh` checks the live
Ahara path after deploy. The Terraform plan path is:

```bash
with-cred -- terraform -chdir=infrastructure/terraform plan -refresh=false -input=false -out=/tmp/linkdrop-m8.tfplan
```
