# Architecture

Bookmarker is an Ahara product repo for personal capture and short-term recall.
The deployed platform key remains `linkdrop`, which provides the public app URL,
API URL, database name, Cognito app client, and Terraform resource prefix.

## Components

| Component                   | Responsibility                                                                                                                                                 |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `android/`                  | Native Android `ACTION_SEND` and `ACTION_SEND_MULTIPLE` client for authenticated URL, text, and image capture.                                                 |
| `backend/api`               | Authenticated Rust HTTP API Lambda behind the shared Ahara ALB.                                                                                                |
| `backend/processing`        | Async URL metadata enrichment Lambda and thumbnail snapshot writer.                                                                                            |
| `backend/shared`            | Shared Rust crate for platform identity, runtime config, Cognito auth, public errors, database helpers, domain types, URL normalization, and library services. |
| `frontend/`                 | Cognito-authenticated Vite React vault plus Electron desktop shell entrypoints for browsing, capture, copy actions, image retrieval, and HUD workflows.        |
| `db/migrations/`            | PostgreSQL migrations for items, URL/text/image payloads, explicit tags, notes, snapshots, processing state, and capture idempotency.                          |
| `infrastructure/terraform/` | Project Terraform root for Ahara website, API, processing, Cognito client, snapshot storage, runtime config, and alarms.                                       |

## Platform integration

The API runs as a Rust Lambda behind the shared Ahara ALB through the
`alb-api` module. `GET /health` is public. Authenticated API routes use the
shared ALB JWT validation action and the shared Cognito pool.

The web app is deployed through the Ahara `website` module. Web, desktop, and
Android clients authenticate through the Linkdrop public Cognito app client
named `linkdrop-app`. The app client is registered with the platform auth
trigger through `/ahara/auth-trigger/clients/linkdrop-app`.

Terraform consumes shared VPC, ALB, Cognito, Route 53, RDS, and state-bucket
context through `ahara-tf-patterns` modules. The backend uses the shared
PostgreSQL/RDS instance with a per-project database named `linkdrop`.

## API surface

| Route                                         | Auth | Contract                                                                                             |
| --------------------------------------------- | ---- | ---------------------------------------------------------------------------------------------------- |
| `GET /health`                                 | No   | Return service health.                                                                               |
| `GET /me`                                     | Yes  | Return the authenticated Cognito user context.                                                       |
| `POST /items`                                 | Yes  | Capture a URL with optional title, explicit tags, and `client_capture_id`.                           |
| `POST /items/text`                            | Yes  | Capture a text snippet with optional title, source metadata, explicit tags, and `client_capture_id`. |
| `POST /items/images/uploads`                  | Yes  | Create an image item and return an authenticated upload target.                                      |
| `POST /items/{item_id}/image-upload/complete` | Yes  | Mark an owned image item as uploaded after object storage receives the bytes.                        |
| `GET /items`                                  | Yes  | Return item summaries scoped to the current user, with optional library filters.                     |
| `GET /items/updates`                          | Yes  | Return changed item summaries and tombstones after a cursor for polling clients.                     |
| `GET /items/{item_id}`                        | Yes  | Return item detail scoped to the current user.                                                       |
| `GET /items/{item_id}/thumbnail`              | Yes  | Return Linkdrop-owned thumbnail snapshot bytes for an owned URL item.                                |
| `GET /items/{item_id}/image`                  | Yes  | Return short-lived presigned uploaded image access URLs for an owned image item.                      |
| `PATCH /items/{item_id}`                      | Yes  | Update title, notes, explicit tags, `watch_status`, and `inbox_status`.                              |
| `DELETE /items/{item_id}`                     | Yes  | Delete an owned item and surface the deletion through update polling.                                |
| `GET /tags`                                   | Yes  | Return the current user's explicit tag corpus.                                                       |
| `PATCH /tags/{tag_id}`                        | Yes  | Rename a tag when the normalized destination is available.                                           |
| `POST /tags/{source_tag_id}/merge`            | Yes  | Merge a source tag into a different target tag.                                                      |

Every authenticated route uses the shared Cognito verifier. Public API errors
use a stable JSON shape with a safe `code` and `message`.

## Item model

`items` is the durable root for every saved object. Payload-specific data lives
in sibling tables:

| Payload      | Table         | Notes                                                                                       |
| ------------ | ------------- | ------------------------------------------------------------------------------------------- |
| URL          | `item_urls`   | Original URL, canonical URL, normalization status, and copy URL source.                     |
| Text snippet | `item_texts`  | Plain text, optional HTML/source metadata, preview text, and content hash.                  |
| Image        | `item_images` | Object key, content type, original filename, byte size, upload status, and source metadata. |

Shared organization data remains on the item: user title, explicit tags, notes,
watched state, inbox state, creation time, update time, and deletion state.
ADRs [0004](adr/0004-general-content-items.md) and
[0005](adr/0005-image-transfer-items.md) record the payload-table decisions.
Captured items land in `inbox_status = unsorted`; users move them to
`organized` after capture. User-entered titles live on `items.title`. Provider
titles from URL enrichment live separately as `metadata_snapshots.title` and
are surfaced to clients as `fetched_title`.

## Capture model

Capture is write-first. URL and text captures persist as soon as the API
validates the required payload field. Image capture creates a pending image item
and returns an upload target; the client uploads the bytes and then completes
the upload. Clients may send a stable `client_capture_id` so retries return the
existing item instead of creating another item.

URL capture stores the original submitted URL and, when normalization succeeds,
a canonical URL. Canonicalization strips common tracking parameters, converts
`youtu.be` without network access, and resolves known share/meta or short-link
hosts through a bounded best-effort resolver. `url.copy_url` is the canonical
URL when available and the original URL when normalization falls back.

The tag corpus is derived only from explicit item-tag associations. Chip
ranking uses tag usage counts from that corpus and starts empty for a new
account.

## Enrichment and archives

URL capture queues the processing Lambda after the database write. Processing
extracts provider or OpenGraph metadata best effort, stores fetched title,
author/channel, platform, optional duration, archive status, and safe error
text in `metadata_snapshots`, and writes thumbnail bytes to a Linkdrop-owned
private snapshot bucket when a thumbnail is available.

The database stores thumbnail snapshot keys and content types. Clients load
snapshots through authenticated API routes that check item ownership before
reading object storage.

## Web and desktop UI

The web UI is a Cognito-authenticated vault workspace for URL, text, and image
items. It renders a table-oriented feed and modal detail surface with title,
source/platform, explicit tags, date added, archive status, watched state,
inbox state, notes, text Markdown rendering, image preview/download, and
thumbnail snapshots.

The item list supports filters for platform, explicit tag, added date range,
archive status, watched state, inbox state, and free-text search over user
title, fetched title, snippet text, filename, and notes. The detail modal
supports click-to-edit titles, blur-saved notes, chip-based tag selection,
status icon popovers, source opening, copy actions, and custom delete
confirmation.

The Electron desktop shell loads the same web app, persists desktop auth state
through the Electron storage boundary, exposes clipboard read/write IPC through
a preload bridge, adds a tray entry for explicit clipboard capture, and
provides a compact always-on-top HUD for recent unsorted items.

## Android UI

The Android app registers share destinations for text and image payloads. Text
shares are parsed for the first HTTP(S) URL; text without a URL is captured as a
text snippet. Image shares create image upload items and stream the Android
content URI to the API-issued upload target. Multiple shared images are captured
as separate image items.

The share screen allows immediate save with optional explicit tag chips and one
free-text tag. The app signs in against the shared Ahara Cognito pool through
the Linkdrop public app client, stores tokens locally, refreshes access tokens
before API calls, and supports the platform software-token MFA challenge flow.

## Operations

The Terraform root creates the website, API Lambda, processing Lambda,
Linkdrop public Cognito client, auth-trigger SSM mapping, private snapshot
bucket, runtime config, and CloudWatch `Errors`/`Throttles` alarms for both
Lambdas. The snapshot bucket is private, blocks public access, uses
server-side encryption, and keeps versioning enabled because thumbnails and
uploaded images are project-owned copies.

Operators use the secret broker for deploy and live smoke commands:

```bash
with-cred -- scripts/deploy.sh
with-cred -- scripts/smoke.sh
```
