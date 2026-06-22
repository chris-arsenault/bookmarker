# API Crate

Buildable Rust HTTP API Lambda crate behind the shared Ahara ALB.

The binary entrypoint is thin: it initializes tracing, builds `ApiState` from
environment configuration, and runs the Axum router through `lambda_http`.
Production state uses shared Cognito auth, the shared PostgreSQL pool, and
`PgLibraryService`. Capture also uses `ProcessingDispatcher` to enqueue
best-effort asynchronous enrichment after the item is saved.

## Routes

| Route | Auth | Response |
| ---- | ---- | ---- |
| `GET /health` | No | Service status |
| `GET /me` | Bearer token | Authenticated `UserContext` |
| `POST /items` | Bearer token | Capture a URL with optional explicit tags and canonical URL deduplication |
| `POST /items/text` | Bearer token | Capture a text snippet with optional explicit tags and content-hash deduplication |
| `GET /items` | Bearer token | Saved item summaries, optionally filtered |
| `GET /items/{item_id}` | Bearer token | Saved item detail |
| `GET /items/{item_id}/thumbnail` | Bearer token | Linkdrop-owned thumbnail snapshot bytes |
| `GET /tags` | Bearer token | Explicit tag corpus entries |
| `PATCH /items/{item_id}` | Bearer token | Updated item detail after item edits |
| `PATCH /tags/{tag_id}` | Bearer token | Updated tag corpus after tag rename |
| `POST /tags/{source_tag_id}/merge` | Bearer token | Updated tag corpus after merging one tag into another |

All authenticated routes use the shared `AuthVerifier` boundary. Public errors
are returned as `{ "code": "...", "message": "..." }`, and CORS headers are
applied by the shared API router layer.

`POST /items` accepts:

```json
{
  "url": "https://example.com/watch",
  "title": "My saved title",
  "tags": ["Learning"],
  "client_capture_id": "android-share-attempt-id"
}
```

`title` and `tags` are optional. `title` stores the user-entered capture title on
the item itself; enrichment writes provider metadata to `fetched_title` instead
of overwriting it. Tags are stored only when the user explicitly supplies them.
`client_capture_id` is optional, but Android uses one stable value per share
attempt so a retry returns the existing item with `200 OK` instead of creating a
duplicate. New captures return `201 Created`.

`POST /items/text` accepts:

```json
{
  "plain_text": "clipboard text to keep nearby",
  "title": "Shell note",
  "html": null,
  "source_app": "Desktop",
  "source_device": "linux",
  "capture_method": "desktop_clipboard",
  "tags": ["Shell"],
  "client_capture_id": "desktop-capture-id"
}
```

Text captures share the same `items`, explicit tags, notes, watched status, and
inbox status as URL captures. They store payload-specific data in `item_texts`,
deduplicate repeated text content by per-user content hash, and return
`archive_status = not_applicable`. Text capture `title` is optional and uses the
same user-entered item title field as URL captures.

URL capture responses include `summary.url.original_url`,
`summary.url.canonical_url` when available, and `summary.url.copy_url`.
`copy_url` is the canonical URL after tracking parameters are stripped and
short-share hosts such as `youtu.be`, `share.google.com`, `vt.tiktok.com`, and
common shorteners are normalized or resolved best effort; if normalization
fails, `copy_url` falls back to the original URL. Repeated captures with the
same canonical URL return the existing item with `200 OK`.

After a created, retry-returned, or deduplicated capture is surfaced, the API
enqueues an `enrich_metadata` processing job. When `PROCESSING_FUNCTION_NAME` is
configured, it invokes the processing Lambda with asynchronous
`InvocationType::Event`; otherwise the queued job remains available for later
processing. Dispatch failures are logged and do not change the HTTP status or
body.

`GET /items` accepts optional query parameters:

| Parameter | Meaning |
| ---- | ---- |
| `platform` | Exact platform filter from `metadata_snapshots.platform` |
| `tag` | Explicit tag corpus key filter |
| `created_from` | RFC3339 lower bound for item creation time |
| `created_to` | RFC3339 upper bound for item creation time |
| `archive_status` | `pending`, `succeeded`, `failed`, or `not_applicable`; URL items without snapshot rows read as `pending` |
| `watch_status` | `unwatched` or `watched` |
| `inbox_status` | `unsorted` or `organized` |
| `q` | Case-insensitive search over user title, fetched title, snippet text, and notes |

API item DTOs expose the user-entered item `title` separately from the
enrichment `fetched_title` stored in `metadata_snapshots`. They also include
`archive_status`, `thumbnail_s3_key`, payload-specific URL/text copy data,
`watch_status`, and `inbox_status`. `PATCH /items/{item_id}`
accepts any non-empty subset of `title`, `watch_status`, `inbox_status`,
`notes`, and `tags`. Missing fields preserve existing values, a blank `title`
clears the user-entered title, `tags: []` clears explicit item tags, and
provided tags are trimmed and deduplicated by normalized name.

`PATCH /tags/{tag_id}` accepts `{ "display_name": "..." }` and rejects
normalized-name collisions. `POST /tags/{source_tag_id}/merge` accepts
`{ "target_tag_id": "..." }`, moves source item associations to the target,
collapses duplicate edges, deletes the source tag, and returns the refreshed
tag corpus. Tag creation and ranking are still driven only by explicit
user-applied tags.

`GET /items/{item_id}/thumbnail` checks the authenticated user's ownership
through the item detail path, reads only the stored snapshot key, and returns
the stored image content type with the snapshot bytes. It does not return source
thumbnail URLs.
