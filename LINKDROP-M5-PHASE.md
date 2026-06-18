# Linkdrop M5 Phase Plan - Async Enrichment and Snapshot Archival

## Scope

M5 adds asynchronous processing for visual metadata and thumbnail snapshots. It does not build the web UI, add thumbnail-read API routes, change Android behavior, implement tag/notes management, or complete Terraform deployment wiring. M8 owns the deployed Lambda module, snapshot bucket resource, IAM grants, alarms, and smoke checks.

Reference behavior comes from `LINKDROP-PLAN.md`, `docs/adr/0001-ahara-platform-topology.md`, `docs/adr/0002-capture-first-async-enrichment.md`, `docs/adr/0003-project-owned-snapshot-storage.md`, and the `../tastebase` async processing pattern:

- Capture remains write-first and never waits on enrichment or thumbnail fetches.
- Capture and retry paths enqueue/invoke processing best effort; dispatch failure must not fail the capture response.
- Processing is idempotent: re-running an item updates the same `metadata_snapshots` and `processing_jobs` rows.
- Metadata enrichment is best-effort. Private, dead, blocked, malformed, or partially parseable sources leave the saved item in place and mark archive status failed where processing reached a terminal failure.
- Original and canonical URLs from M4 remain the source identity; processing prefers canonical URL when present and falls back to original URL.
- Thumbnails are copied into Linkdrop-owned storage and persisted as `thumbnail_s3_key`; source thumbnail URLs are not stored as hotlinks.
- API list/detail responses already expose `archive_status` and `thumbnail_s3_key` through the existing item DTO shape.
- The existing M1 schema already provides `metadata_snapshots` and `processing_jobs`; M5 should use those tables rather than adding a new schema shape.

The M5 exit gate is `make ci` green; processing is idempotent and failed enrichment leaves a saved item with visible failed archive status.

## Steps

1. Add the buildable processing Lambda crate

   File(s): `backend/Cargo.toml`, `backend/processing/Cargo.toml`, `backend/processing/src/main.rs`, `backend/processing/README.md`

   Reference behavior: Ahara async work follows the `../tastebase/backend/processing` pattern: a workspace crate with a thin `lambda_runtime` entrypoint, typed event payload, structured tracing, and runtime config loaded from shared environment. M5 only adds the code artifact; Terraform registration stays in M8.

   Change: register `backend/processing` as a Rust workspace member, add the required workspace dependencies (`lambda_runtime`, `aws-config`, `aws-sdk-s3`, `aws-sdk-lambda`, and any HTML parser dependency used by later M5 steps), and replace the reserved README-only directory with a minimal compiling Lambda crate. Define a typed processing event carrying `item_id` and a small validation path for malformed events.

   Verify: first confirm the crate/test contract is absent with `test ! -f backend/processing/Cargo.toml` and `rg "processing_event_requires_item_id" backend/processing`, then add and run:

   ```sh
   test -f backend/processing/Cargo.toml
   rg "processing_event_requires_item_id" backend/processing
   cd backend && cargo test -p processing processing_event_requires_item_id
   ```

2. Add shared processing persistence for queueing, status, and snapshots [depends on #1]

   File(s): `backend/shared/src/processing.rs`, `backend/shared/src/lib.rs`, `backend/shared/Cargo.toml`, `backend/shared/tests/linkdrop_processing.rs`

   Reference behavior: `db/migrations/001_create_linkdrop_model.sql` already defines `processing_jobs` and `metadata_snapshots` with idempotent keys and valid status values. ADR 0002 requires failed processing to be visible without deleting the saved item. M4 canonical URLs remain stored in `item_urls`.

   Change: add a shared processing repository/service for:
   - enqueueing or requeueing an `enrich_metadata` job for a user-owned item,
   - loading the item URL and user id for processing by item id,
   - marking `enrich_metadata` and `snapshot_thumbnail` jobs running/succeeded/failed,
   - upserting `metadata_snapshots` as pending/succeeded/failed without creating duplicate rows.

   Do not add migrations. The repository should use deterministic idempotency keys such as `{job_kind}:{item_id}` and preserve existing succeeded snapshot data unless a rerun produces a newer successful snapshot.

   Verify: first confirm the repository/test contract is absent, then add and run:

   ```sh
   rg "ProcessingRepository|processing_repository_queues_and_retries_enrichment_jobs" backend/shared
   cd backend && cargo test --workspace --test linkdrop_processing processing_repository_queues_and_retries_enrichment_jobs
   ```

3. Dispatch processing from capture and retry paths [depends on #2]

   File(s): `backend/api/Cargo.toml`, `backend/api/src/lib.rs`, `backend/api/src/item_routes.rs`, `backend/api/src/processing_dispatch.rs`, `backend/api/tests/api_capture.rs`, `backend/api/tests/support/mod.rs`

   Reference behavior: capture must not block on enrichment. The `../tastebase` API dispatch pattern invokes Lambda asynchronously with `InvocationType::Event` when a function name is configured. Linkdrop must dispatch for both newly created captures and existing items surfaced by `client_capture_id` retry or canonical dedup, because a repeat capture is also the user asking for the item to be present and processable.

   Change: add a `ProcessingDispatcher` trait to the API layer. Production dispatch should enqueue the shared processing job and, when `PROCESSING_FUNCTION_NAME` is set, invoke the processing Lambda asynchronously with the item id payload. When the env var is absent, dispatch is a no-op after queueing. Dispatch errors should be logged/recorded but must not change the HTTP capture status or body. Inject a fake dispatcher in API tests.

   Verify: first confirm the API dispatch tests are absent, then add and run:

   ```sh
   rg "processing_dispatch_runs_for_created_and_retry_captures|processing_dispatch_failure_does_not_block_capture" backend/api/tests/api_capture.rs
   cd backend && cargo test -p api --test api_capture processing_dispatch
   ```

4. Add best-effort metadata extractors [depends on #1]

   File(s): `backend/processing/src/extractors.rs`, `backend/processing/src/main.rs`, `backend/processing/Cargo.toml`

   Reference behavior: M5 fetches title, thumbnail, author/channel, platform, and duration best effort. Provider-specific sources should be tried where practical, with generic OpenGraph/metadata parsing as the fallback. Duration is optional when unavailable. Extractors must not infer tags.

   Change: add metadata result types and extractor code for:
   - platform detection from canonical/original URL host,
   - OpenGraph/meta parsing for title, thumbnail URL, site/platform, author, and duration where present,
   - provider hooks for known video hosts such as YouTube/TikTok that can use provider/oEmbed metadata when available,
   - graceful extractor errors that distinguish "no metadata found" from HTTP/parse failures.

   Tests should use static HTML/JSON fixtures and fake fetch responses; no test should call live provider URLs.

   Verify: first confirm the extractor tests are absent, then add and run:

   ```sh
   rg "OpenGraphExtractor|extracts_opengraph_video_metadata|extracts_provider_oembed_metadata" backend/processing/src
   cd backend && cargo test -p processing extractors
   ```

5. Add thumbnail snapshot storage boundary [depends on #4]

   File(s): `backend/processing/src/snapshot_store.rs`, `backend/processing/src/main.rs`, `backend/processing/Cargo.toml`

   Reference behavior: ADR 0003 requires Linkdrop-owned private thumbnail copies, not source hotlinks. M5 code should support S3 writes through `SNAPSHOT_BUCKET`, but M8 owns creating the bucket and IAM. Snapshot keys must be deterministic by item id so reruns overwrite the same owned snapshot instead of accumulating duplicates.

   Change: add a `ThumbnailStore` boundary with an S3 implementation and fake implementation for tests. The S3 implementation should download thumbnail bytes through the existing HTTP client path, write them to `snapshots/{item_id}/thumbnail.{ext-or-bin}`, set content type, and return only the Linkdrop-owned key/content type. It must not persist or expose the source thumbnail URL.

   Verify: first confirm the storage test is absent, then add and run:

   ```sh
   rg "ThumbnailStore|stores_thumbnail_snapshot_without_hotlink" backend/processing/src
   cd backend && cargo test -p processing snapshot_store::tests::stores_thumbnail_snapshot_without_hotlink
   ```

6. Implement the idempotent processing pipeline [depends on #2, #4, #5]

   File(s): `backend/processing/src/main.rs`, `backend/processing/src/extractors.rs`, `backend/processing/src/snapshot_store.rs`, `backend/processing/tests/processing_pipeline.rs`

   Reference behavior: ADR 0002 says failed enrichment is visible but never deletes or rejects a saved link. ADR 0003 says successful thumbnails are stored as Linkdrop-owned keys. The existing API item queries read `metadata_snapshots.archive_status`, so writing that row is enough for visible pending/succeeded/failed status.

   Change: wire the Lambda handler to:
   - load the item and preferred source URL by `item_id`,
   - mark the enrichment job running and increment attempts,
   - extract metadata best effort,
   - mark snapshot processing running when a thumbnail URL exists,
   - copy the thumbnail into Linkdrop-owned storage and persist `thumbnail_s3_key`,
   - upsert metadata fields and `archive_status = succeeded` on success,
   - upsert partial metadata and `archive_status = failed` with a safe error on extraction or snapshot failure,
   - mark processing jobs succeeded/failed idempotently.

   Reruns should update the same rows. A failed source should leave the item queryable with `archive_status = failed`.

   Verify: first confirm the pipeline test is absent, then add and run:

   ```sh
   rg "processing_pipeline_records_snapshot_success_and_failure" backend/processing/tests
   cd backend && cargo test -p processing --test processing_pipeline processing_pipeline_records_snapshot_success_and_failure
   ```

7. Document the M5 processing contract [depends on #6]

   File(s): `README.md`, `backend/README.md`, `backend/api/README.md`, `backend/processing/README.md`, `docs/architecture.md`, `CHANGELOG.md`

   Reference behavior: docs should describe only the M5 backend processing contract now present. Do not claim the web feed, thumbnail read URLs, Terraform bucket, alarms, deploy registration, or tag/notes workflows are complete.

   Change: update docs to state that capture queues asynchronous processing, processing enriches metadata best effort, thumbnails are stored as Linkdrop-owned snapshot keys, archive status becomes pending/succeeded/failed, and failed enrichment leaves the saved item visible.

   Verify: confirm docs mention the M5 contract and do not claim future phases are done:

   ```sh
   rg "processing Lambda|metadata_snapshots|snapshot bucket|archive_status|thumbnail_s3_key|best-effort" README.md backend/README.md backend/api/README.md backend/processing/README.md docs/architecture.md CHANGELOG.md
   ! rg "web library UI is implemented|Terraform snapshot bucket is deployed|thumbnail read endpoint is implemented|tag rename is implemented" README.md backend/README.md backend/api/README.md backend/processing/README.md docs/architecture.md CHANGELOG.md
   ```

## Exit Gate

Run the canonical repository gate from the repository root:

```sh
make ci
```

The phase is complete only when `make ci` is green and the M5-specific tests demonstrate:

- capture and retry paths dispatch processing without blocking capture,
- queued processing jobs are idempotent and retryable,
- metadata extraction handles provider and OpenGraph fixtures without live network calls,
- thumbnail snapshots store Linkdrop-owned keys rather than source hotlinks,
- processing success writes snapshot metadata and archive status,
- processing failure leaves the item saved and visible with failed archive status.
