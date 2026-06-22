# Backend

Rust workspace for Linkdrop backend code.

The registered workspace members are `shared`, `api`, and `processing`.

| Crate | Purpose |
| ---- | ---- |
| `shared` | Shared config, auth, public errors, database helpers, M1 migration SQL constants, domain validation, URL normalization, processing persistence, and library service boundary |
| `api` | Authenticated HTTP API Lambda behind the shared Ahara ALB with best-effort async processing dispatch |
| `processing` | Async metadata enrichment Lambda with Linkdrop-owned thumbnail snapshot storage boundary |

`shared::db` exposes the M1 migration and rollback SQL for PostgreSQL tests.
`shared::domain` defines the typed boundary used by later backend crates,
including `SubmittedUrl`, `TagName`, `ArchiveStatus`, `WatchStatus`,
`ProcessingJobKind`, and `ProcessingStatus`.
`shared::library` defines item/tag reads, quick-drop capture, organization
mutations, and tag cleanup; `shared::library_pg` implements those operations
over the project schema. `shared::processing` owns idempotent
`processing_jobs` queue updates and `metadata_snapshots` upserts for the
processing Lambda.

## API surface

| Route | Auth | Purpose |
| ---- | ---- | ---- |
| `GET /health` | No | Service health response |
| `GET /me` | Yes | Current Cognito user context |
| `POST /items` | Yes | Capture a URL with optional explicit tags and optional `client_capture_id` |
| `GET /items` | Yes | Current user's saved item summaries with library filters |
| `GET /items/{item_id}` | Yes | Current user's saved item detail |
| `GET /items/{item_id}/thumbnail` | Yes | Current user's Linkdrop-owned snapshot thumbnail |
| `GET /tags` | Yes | Current user's explicit tag corpus |
| `PATCH /items/{item_id}` | Yes | Update title, notes, explicit tags, `watch_status`, and `inbox_status` |
| `PATCH /tags/{tag_id}` | Yes | Rename a tag when the normalized destination does not collide |
| `POST /tags/{source_tag_id}/merge` | Yes | Merge a source tag into a different target tag |

`POST /items` stores the original URL immediately, applies only user-supplied
explicit tags, and uses `client_capture_id` to make Android share retries safe.
It also stores a canonical URL when normalization succeeds, strips tracking
parameters, normalizes `youtu.be`, resolves `vt.tiktok.com` best effort,
deduplicates repeated canonical URLs, and returns `copy_url` for client copy
actions. After the database write succeeds, the API enqueues processing and
invokes the processing Lambda asynchronously when `PROCESSING_FUNCTION_NAME` is
configured. Dispatch failures are logged but do not fail capture.

The processing Lambda fetches metadata best effort, stores thumbnail bytes via a
Linkdrop-owned snapshot key in `thumbnail_s3_key`, and writes
`archive_status` as `succeeded` or `failed`. Failed enrichment keeps the saved
item visible. Thumbnail reads are API-mediated and require `SNAPSHOT_BUCKET` in
the API runtime environment. Terraform wires the snapshot bucket,
`PROCESSING_FUNCTION_NAME`, and CloudWatch Lambda alarms through the Ahara
deployment modules.
