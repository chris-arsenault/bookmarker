# M3 — Quick-Drop Capture and Android Share Target Phase Plan

This expands only `M3 — Quick-Drop Capture and Android Share Target` from
[LINKDROP-PLAN.md](LINKDROP-PLAN.md). M3 adds authenticated quick-drop capture,
explicit capture-time tags, and the native Android share target loop. It does
not implement URL normalization, canonical URL deduplication, async enrichment,
thumbnail storage, offline queueing, web library workflows, tag rename/merge,
notes editing, or Terraform API resources.

## Reference Context

- Plan context/reuse map: [LINKDROP-PLAN.md](LINKDROP-PLAN.md)
- Current architecture and API contract:
  [docs/architecture.md](docs/architecture.md), [LINKDROP-M2-PHASE.md](LINKDROP-M2-PHASE.md)
- Capture-first processing decision:
  [docs/adr/0002-capture-first-async-enrichment.md](docs/adr/0002-capture-first-async-enrichment.md)
- Ahara platform topology:
  [docs/adr/0001-ahara-platform-topology.md](docs/adr/0001-ahara-platform-topology.md)
- M1 schema/domain contracts: [LINKDROP-M1-PHASE.md](LINKDROP-M1-PHASE.md),
  `db/migrations/001_create_linkdrop_model.sql`
- Ahara API/auth/error reference: `../ahara-business/backend/api/src/lib.rs`,
  `../ahara-business/backend/api/src/main.rs`, `../ahara-business/backend/api/src/cors.rs`
- Ahara platform database/API contract: `../ahara/INTEGRATION.md` Steps 4 and 5
- Standards: `../ahara-standards/standards/rust.md`,
  `testing.md`, `project-structure.md`

Android verification note: M0 intentionally omitted Android compile checks
because the repo has no Gradle wrapper, Java/Gradle are not available in this
environment, and Ahara does not yet have an Android CI standard. M3 therefore
adds a structural Android gate to `make ci` for the share-target contract. A
full Gradle/JDK compile gate should be introduced when the platform standard is
chosen.

## Steps

1. Add capture retry idempotency to the database model.
   - File(s): `db/migrations/002_capture_idempotency.sql`,
     `db/migrations/rollback/002_capture_idempotency.sql`,
     `backend/shared/src/db.rs`, `backend/shared/tests/linkdrop_capture_idempotency.rs`
   - Reference behavior: ADR 0002 requires capture to persist immediately, and
     M3 requires Android retry-safe request handling. M4 owns URL
     normalization and canonical deduplication, so M3 idempotency must be keyed
     to a client-generated capture attempt, not to the URL.
   - Change: add an optional nonempty `items.client_capture_id` column with a
     partial unique index on `(user_id, client_capture_id)` when present; expose
     the migration and rollback SQL from `shared::db`; add a PostgreSQL
     integration test proving the same user/client capture id cannot create two
     items while the same URL without a client capture id is still not
     canonical-deduped in M3.
   - Verify: `rg "002_capture_idempotency" backend/shared/src/db.rs && rg "fn linkdrop_capture_idempotency_prevents_duplicate_share_retries" backend/shared/tests/linkdrop_capture_idempotency.rs && cd backend && cargo test --workspace --test linkdrop_capture_idempotency linkdrop_capture_idempotency_prevents_duplicate_share_retries`. Red before: the migration constant and integration test do not exist.

2. Add the capture request boundary and in-memory implementation. [depends on #1]
   - File(s): `backend/shared/src/library.rs`, `backend/shared/Cargo.toml`
   - Reference behavior: M3 capture accepts a URL plus optional explicit tags,
     with zero user-mandatory metadata fields. `SubmittedUrl` validates the
     submitted URL without normalizing it; `TagName` accepts only explicit
     user-applied tags. M4 owns canonicalization and URL deduplication. M7 owns
     later tag editing and tag rename/merge workflows.
   - Change: add `CaptureItemRequest` with required `url`, default-empty
     `tags`, and optional `client_capture_id`; add `CaptureItemOutcome` with
     the saved `LibraryItemDetail` and `created` flag; add
     `LibraryService::capture_item`; update `InMemoryLibraryService` to persist
     a new unwatched item with `original_url`, `canonical_url: None`, pending
     archive status, empty notes, and only explicitly supplied tags. Repeated
     in-memory captures with the same `client_capture_id` for the same user
     return the existing item rather than creating another one.
   - Verify: `rg "pub struct CaptureItemRequest" backend/shared/src/library.rs && rg "capture_item" backend/shared/src/library.rs && cd backend && cargo test --workspace --lib library::tests::in_memory_capture`. Red before: the capture DTOs, service method, and named tests do not exist.

3. Implement PostgreSQL no-tag capture persistence. [depends on #1] [depends on #2]
   - File(s): `backend/shared/src/library_pg.rs`,
     `backend/shared/tests/support/mod.rs`, `backend/shared/tests/library_pg_capture.rs`
   - Reference behavior: Production API state uses Ahara's shared PostgreSQL
     database through the M1 schema and authenticated user ownership. ADR 0002
     says capture saves even before normalization/enrichment. M5 owns processing
     invocation, so this step must not enqueue processing jobs or fetch
     metadata.
   - Change: implement `PgLibraryService::capture_item` for requests with no
     tags: upsert the authenticated user by `cognito_sub`, run the item and URL
     inserts in a transaction, use `client_capture_id` for idempotent retry when
     present, store only `original_url`, leave `canonical_url` null, leave
     metadata snapshot rows absent, and return the same detail shape used by
     item reads. Extend shared PostgreSQL test support only as needed for sqlx
     integration tests.
   - Verify: `rg "fn pg_capture_persists_original_url_without_normalization" backend/shared/tests/library_pg_capture.rs && cd backend && cargo test --workspace --test library_pg_capture pg_capture_persists_original_url_without_normalization`. Red before: `PgLibraryService::capture_item` and the named integration test do not exist.

4. Apply explicit capture tags and corpus counts in PostgreSQL. [depends on #3]
   - File(s): `backend/shared/src/library_pg.rs`,
     `backend/shared/tests/library_pg_capture.rs`
   - Reference behavior: The tag corpus starts empty and contains only tags the
     user explicitly applies. M1 owns tag usage triggers; M3 must reuse those
     triggers instead of inventing separate counters. Existing tag display names
     are preserved; M7 owns rename/merge cleanup.
   - Change: extend `PgLibraryService::capture_item` to trim and validate
     supplied tag strings with `TagName`, insert missing tags for the current
     user, reuse existing tags by normalized name without renaming them, insert
     `item_tags` with `applied_source = 'explicit'` and `ON CONFLICT DO
     NOTHING`, and return item tags plus corpus ordering by usage count
     descending then normalized name ascending.
   - Verify: `rg "fn pg_capture_applies_only_explicit_tags_and_updates_corpus" backend/shared/tests/library_pg_capture.rs && cd backend && cargo test --workspace --test library_pg_capture pg_capture_applies_only_explicit_tags_and_updates_corpus`. Red before: explicit capture-tag persistence is not implemented or covered.

5. Add the authenticated capture API route. [depends on #2] [depends on #4]
   - File(s): `backend/api/src/item_routes.rs`, `backend/api/tests/api_capture.rs`,
     `backend/api/tests/support/mod.rs`, `backend/api/README.md`
   - Reference behavior: Every backend route except health uses shared Cognito
     auth, the M2 API returns structured errors, and M3 capture must accept a
     URL with optional explicit tags while leaving normalization/dedup/enrichment
     to later phases.
   - Change: add `POST /items` accepting `CaptureItemRequest`, require auth,
     return `201 Created` when a new item is created and `200 OK` when
     `client_capture_id` returns an existing item, and map validation/database
     failures through `ApiError`. Add API tests for missing auth, invalid URL
     validation shape, URL-only capture, explicit tags, and idempotent retry
     returning one library item.
   - Verify: `rg "post\\(capture_item\\)" backend/api/src/item_routes.rs && rg "fn capture_route_accepts_url_without_tags_and_lists_item" backend/api/tests/api_capture.rs && cd backend && cargo test -p api --test api_capture`. Red before: the capture route and API test target do not exist.

6. Add Android runtime config and authenticated token boundary.
   - File(s): `android/app/build.gradle.kts`,
     `android/app/src/main/AndroidManifest.xml`,
     `android/app/src/main/java/io/ahara/linkdrop/config/LinkdropConfig.kt`,
     `android/app/src/main/java/io/ahara/linkdrop/auth/AuthRepository.kt`,
     `android/app/src/main/java/io/ahara/linkdrop/auth/AuthTokenStore.kt`,
     `android/app/src/main/java/io/ahara/linkdrop/MainActivity.kt`
   - Reference behavior: ADR 0001 requires Android to consume the same
     authenticated API as web through the shared Cognito pool and Linkdrop app
     client. The share target must not send unauthenticated capture requests.
   - Change: add Android dependencies and BuildConfig/resource plumbing needed
     for API base URL, Cognito issuer/domain, client id, and redirect URI; add a
     token store that persists access/refresh token metadata; add an auth
     repository boundary for obtaining a fresh bearer token before API calls;
     update the launcher activity to provide the sign-in/auth-status surface.
   - Verify: `rg "AuthRepository" android/app/src/main/java/io/ahara/linkdrop && rg "AuthTokenStore" android/app/src/main/java/io/ahara/linkdrop && rg "LINKDROP_API_BASE_URL|COGNITO" android/app/build.gradle.kts android/app/src/main/java/io/ahara/linkdrop`. Red before: Android auth/config boundaries do not exist.

7. Add the Android Linkdrop API client with idempotent capture requests. [depends on #5] [depends on #6]
   - File(s): `android/app/build.gradle.kts`,
     `android/app/src/main/java/io/ahara/linkdrop/api/LinkdropApiClient.kt`,
     `android/app/src/main/java/io/ahara/linkdrop/api/LinkdropApiModels.kt`
   - Reference behavior: Android uses the same M3 authenticated API as web.
     Retry safety is provided by reusing the same `client_capture_id` for one
     share attempt; M3 must not rely on URL deduplication because M4 owns that.
     Offline queueing is backlog work, not M3.
   - Change: add a small HTTP client that calls `GET /tags` and `POST /items`
     with `Authorization: Bearer <access token>`; serialize `url`, `tags`, and
     `client_capture_id`; generate one stable capture id per share attempt; use
     the same id for explicit retry after network failure; do not automatically
     retry ambiguous POST failures with a different id.
   - Verify: `rg "class LinkdropApiClient" android/app/src/main/java/io/ahara/linkdrop/api && rg "client_capture_id" android/app/src/main/java/io/ahara/linkdrop/api && rg "\"/items\"|\"/tags\"" android/app/src/main/java/io/ahara/linkdrop/api`. Red before: Android API client files and capture id serialization do not exist.

8. Add the native Android share target and zero-required-fields capture flow. [depends on #7]
   - File(s): `android/app/src/main/AndroidManifest.xml`,
     `android/app/src/main/java/io/ahara/linkdrop/share/ShareActivity.kt`,
     `android/app/src/main/java/io/ahara/linkdrop/share/ShareIntentParser.kt`,
     `android/app/src/main/res/values/strings.xml`
   - Reference behavior: M3 delivers the Android `ACTION_SEND` share target.
     Quick-drop has no mandatory user-entered fields: the shared URL is the only
     required payload, tags are optional, and confirmation is a toast that does
     not block the share flow.
   - Change: register an exported share activity for `ACTION_SEND` text/plain
     payloads; parse URLs from `Intent.EXTRA_TEXT`; reject missing/invalid URL
     payloads with a non-blocking toast; show an immediately enabled drop action
     with no required fields; call the API client with the parsed URL, optional
     selected tags, and the per-attempt `client_capture_id`; finish the share
     activity after success or explicit cancel.
   - Verify: `rg "android.intent.action.SEND" android/app/src/main/AndroidManifest.xml && rg "text/plain" android/app/src/main/AndroidManifest.xml && rg "class ShareActivity" android/app/src/main/java/io/ahara/linkdrop/share && rg "ShareIntentParser" android/app/src/main/java/io/ahara/linkdrop/share`. Red before: the manifest share target and share activity/parser do not exist.

9. Add optional share-time tag chips and free-text tags. [depends on #7] [depends on #8]
   - File(s): `android/app/src/main/java/io/ahara/linkdrop/share/ShareActivity.kt`,
     `android/app/src/main/java/io/ahara/linkdrop/share/ShareTagState.kt`,
     `android/app/src/main/java/io/ahara/linkdrop/share/TagChipRow.kt`
   - Reference behavior: The tag corpus starts empty and is built only from
     explicit tags. M3 share-time chips use the backend `GET /tags` ranking;
     no tags are inferred or generated. M7 owns later tag editing and
     rename/merge workflows.
   - Change: load tag corpus for the authenticated user, render most-used tags
     as tappable chips when present, render no placeholder chips for an empty
     corpus, allow free-text tag entry, de-duplicate selected tags by normalized
     name before capture, and send only selected/free-text tags in the capture
     request.
   - Verify: `rg "ShareTagState" android/app/src/main/java/io/ahara/linkdrop/share && rg "TagChipRow" android/app/src/main/java/io/ahara/linkdrop/share && rg "listTags|/tags" android/app/src/main/java/io/ahara/linkdrop/share android/app/src/main/java/io/ahara/linkdrop/api`. Red before: share-time tag state and chip rendering do not exist.

10. Add Android structural verification to the local gate. [depends on #6] [depends on #7] [depends on #8] [depends on #9]
    - File(s): `Makefile`, `scripts/check-android-share-target.sh`,
      `docs/development.md`
    - Reference behavior: `make ci` is the canonical local verification command.
      M0 documented that Android compile checks would join only when a platform
      build standard exists; M3 now adds native share-target behavior, but this
      repo still lacks Java/Gradle/wrapper availability.
    - Change: add a lightweight `android-structure-check` target to `make ci`
      that verifies the manifest share target, auth boundary, API client,
      `client_capture_id` serialization, tag corpus usage, and absence of
      generated/inferred tag behavior. Document that this is a structural gate,
      not a Gradle compile replacement.
    - Verify: `rg "android-structure-check" Makefile && test -x scripts/check-android-share-target.sh && make android-structure-check`. Red before: the target and script do not exist.

11. Refresh M3 capture and Android documentation.
    - File(s): `README.md`, `backend/README.md`, `backend/api/README.md`,
      `android/README.md`, `docs/architecture.md`, `docs/development.md`,
      `docs/backlog.md`
    - Reference behavior: Repo docs describe current-state contracts. After M3,
      quick-drop capture, explicit capture-time tags, Android `ACTION_SEND`, and
      retry-safe `client_capture_id` are current behavior; URL normalization,
      canonical deduplication, enrichment, thumbnail archival, offline queueing,
      web library UI, and tag management remain later work.
    - Change: document `POST /items`, capture request/response semantics,
      idempotent `client_capture_id`, explicit-tag-only corpus updates, Android
      share target behavior, auth prerequisites, and Android structural CI
      limitations. Move Android offline queueing out of any current-state text
      and keep it in backlog.
    - Verify: `rg "POST /items|ACTION_SEND|client_capture_id|explicit tag" README.md backend/README.md backend/api/README.md android/README.md docs/architecture.md && ! rg "URL normalization is implemented|canonical deduplication is implemented|enrichment is implemented|thumbnail archival is implemented|offline queueing is implemented|tag rename is implemented|web library UI is implemented" README.md docs backend android`. Red before: docs do not describe the M3 capture/Android route surface.

## Exit Gate

Run after all steps:

```bash
make ci
```

The phase is complete when `make ci` is green, authenticated `POST /items`
capture is covered by API and PostgreSQL tests, explicit capture-time tags
update the corpus from user-supplied tags only, Android share-target structure
is checked by the local gate, and Android uses `client_capture_id` for
retry-safe capture attempts without starting M4 normalization/deduplication or
M5 enrichment.
