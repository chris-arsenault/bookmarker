# Linkdrop M6 Phase Plan - Web Library Interface

## Scope

M6 builds the authenticated web viewing surface for the library. It adds the
minimal backend/API support the web UI needs for filtering, notes search, and
API-mediated thumbnail reads, then implements the React app shell, API client,
visual feed, item detail, copy-link action, source deep-link, archive status,
and responsive filter controls.

M6 does not add capture UI, Android behavior, tag editing, notes editing,
watched-state bulk workflows, tag rename/merge, saved searches, Terraform
deployment wiring, alarms, or provider-specific inline embeds. For the
`inline playback or source deep-link` milestone option, M6 implements source
deep-link because the current backend contract exposes canonical/live source
URLs but not provider embed URLs.

Reference behavior comes from `LINKDROP-PLAN.md`,
`docs/adr/0001-ahara-platform-topology.md`,
`docs/adr/0002-capture-first-async-enrichment.md`,
`docs/adr/0003-project-owned-snapshot-storage.md`,
`../ahara-standards/standards/typescript.md`,
`../ahara-standards/standards/testing.md`, and the
`../ahara-business/frontend` auth/API-client patterns:

- Web uses the same shared Cognito app client as Android and calls the same
  authenticated API.
- Capture remains write-first; the web UI only browses and organizes what
  already exists.
- Item lists/details tolerate pending, succeeded, and failed enrichment.
- Client copy actions use the canonical `copy_url` already exposed by the API.
- Thumbnail display must use Linkdrop-owned snapshot access and must not
  hotlink source thumbnail URLs.
- Filters include platform, explicit tag, added date, archive status, watched
  status, and free-text search over title/notes.
- Tags are displayed from explicit user-applied tags only; tag editing and
  rename/merge are M7.
- TypeScript files stay under 400 lines, functions under 75 lines, and
  cognitive complexity under 10; views should be split by component/pure helper
  rather than growing a giant `App.tsx`.

The M6 exit gate is `make ci` green; the web UI supports authenticated browsing,
filtering, item detail, API-mediated thumbnail display when a snapshot exists,
source deep-link, and canonical copy-link.

## Steps

1. Add the shared item-list filter contract

   File(s): `backend/shared/src/library.rs`,
   `backend/shared/src/library_in_memory.rs`,
   `backend/shared/src/library_tests.rs`

   Reference behavior: M6 filtering must be scoped to the current user's
   library and must not mutate tags, notes, watch state, or capture data. The
   existing in-memory library backs API tests, so it must match the production
   filter semantics before route code is added. Free-text search includes title
   and notes; tag filters use explicit item tags.

   Change: add a `ListItemsQuery` or equivalent shared request type with
   optional platform, tag, created-from, created-to, archive status, watch
   status, and text query fields. Update `LibraryService::list_items` to accept
   the query, and implement the same filtering in `InMemoryLibraryService`.
   Preserve the existing empty-query behavior and existing sort order.

   Verify: first confirm the filter contract/test is absent, then add and run:

   ```sh
   rg "ListItemsQuery|in_memory_list_filters_items_by_status_tag_date_and_text" backend/shared/src
   cd backend && cargo test --workspace --lib library::tests::in_memory_list_filters_items_by_status_tag_date_and_text
   ```

2. Apply item-list filters in PostgreSQL and the HTTP route [depends on #1]

   File(s): `backend/shared/src/library_pg.rs`,
   `backend/shared/src/library_pg_rows.rs`,
   `backend/shared/tests/library_pg_items.rs`,
   `backend/api/src/item_routes.rs`, `backend/api/tests/api_items.rs`

   Reference behavior: the database query remains user-scoped. Platform comes
   from `metadata_snapshots.platform`; archive status defaults to pending when a
   snapshot row is missing; watched status comes from `items.watch_status`; tag
   filters join explicit `item_tags`/`tags`; text search covers
   `metadata_snapshots.title` and `item_notes.body`. Invalid enum query values
   should return the existing safe validation error shape instead of panicking.

   Change: map `GET /items` query parameters into the shared list query and
   apply the filters in `PgLibraryService`. Add focused PostgreSQL tests for
   platform/tag/date/archive/watch/text filtering and route tests for query
   parsing and response shape. Keep `GET /items/{item_id}` unchanged.

   Verify: first confirm the database/API filter tests are absent, then add and
   run:

   ```sh
   rg "pg_list_items_filters_by_metadata_tag_status_and_notes|items_route_applies_library_filters" backend/shared/tests backend/api/tests
   cd backend && cargo test --workspace --test library_pg_items pg_list_items_filters_by_metadata_tag_status_and_notes
   cd backend && cargo test -p api --test api_items items_route_applies_library_filters
   ```

3. Add authenticated thumbnail snapshot read access [depends on #1]

   File(s): `backend/api/Cargo.toml`, `backend/api/src/lib.rs`,
   `backend/api/src/item_routes.rs`, `backend/api/src/thumbnail_access.rs`,
   `backend/api/tests/api_thumbnails.rs`, `backend/api/tests/support/mod.rs`

   Reference behavior: ADR 0003 requires API-mediated access to
   Linkdrop-owned private thumbnails. M5 stores only `thumbnail_s3_key` and
   content type; clients must never receive or hotlink source thumbnail URLs.
   M8 owns creating the bucket, IAM grants, and deployed runtime wiring, but M6
   can add the code boundary and fake-backed tests.

   Change: add a `ThumbnailReader` trait, fake test implementation, and S3
   production implementation that reads the configured snapshot bucket. Add
   `GET /items/{item_id}/thumbnail`, requiring normal Cognito auth and checking
   ownership through the existing item detail path before reading the stored
   key. Return image bytes with the stored content type when a key exists, and a
   safe not-found response when the item has no thumbnail snapshot.

   Verify: first confirm the thumbnail route/test contract is absent, then add
   and run:

   ```sh
   rg "ThumbnailReader|item_thumbnail_route_returns_owned_snapshot" backend/api/src backend/api/tests
   cd backend && cargo test -p api --test api_thumbnails item_thumbnail_route_returns_owned_snapshot
   ```

4. Add frontend auth foundation

   File(s): `frontend/src/config.ts`, `frontend/src/auth.ts`,
   `frontend/src/auth.test.ts`

   Reference behavior: Ahara web clients use the shared Cognito app client and
   expose a small browser auth boundary with session initialization, sign-in,
   logout, token refresh, and testable adapters. The current Linkdrop config
   has a raw optional global that already warns under Ahara TypeScript
   standards; M6 should adopt the `RuntimeGlobal<T>` pattern used in
   `../ahara-business/frontend/src/config.ts`.

   Change: update runtime config typing without changing the public config
   names, and add a Linkdrop auth client adapted from the Ahara Business
   pattern. Keep the Cognito SDK behind an adapter so tests use no live Cognito
   calls.

   Verify: first confirm the auth client/test is absent, then add and run:

   ```sh
   rg "createAuthClient|auth_client_reports_signed_out_without_session" frontend/src
   cd frontend && pnpm exec vitest run src/auth.test.ts
   ```

5. Add the typed frontend API client [depends on #4]

   File(s): `frontend/src/apiCore.ts`, `frontend/src/api.ts`,
   `frontend/src/types.ts`, `frontend/src/api.test.ts`

   Reference behavior: Ahara TypeScript standards forbid scattered direct
   `fetch`; authenticated HTTP must flow through a shared wrapper that attaches
   bearer tokens, refreshes once on `401`, and maps safe API errors. Linkdrop
   client types should mirror the existing Rust DTOs: item summaries/details,
   tag corpus entries, archive/watch status, `copy_url`, `thumbnail_s3_key`,
   and list filters.

   Change: add a reusable `authenticatedRequest` wrapper and `ApiClient` with
   `listItems(filters)`, `getItem(itemId)`, `listTags()`,
   `updateWatchStatus(itemId, watchStatus)`, and
   `fetchThumbnail(itemId)` methods. Serialize filters as the route query
   parameters from step 2. Keep thumbnail reads authenticated and blob-based so
   `<img>` elements do not need bearer headers.

   Verify: first confirm the API client/test is absent, then add and run:

   ```sh
   rg "class ApiClient|api_client_serializes_library_filters_and_auth" frontend/src
   cd frontend && pnpm exec vitest run src/api.test.ts
   ```

6. Add library state and filter view-model helpers [depends on #5]

   File(s): `frontend/src/libraryState.ts`, `frontend/src/libraryFilters.ts`,
   `frontend/src/libraryState.test.ts`, `frontend/src/libraryFilters.test.ts`

   Reference behavior: the UI must handle loading, signed-out, empty, error,
   pending archive, failed archive, and populated states without scattering
   ad hoc loading/error flags through view components. Filter chips/options
   should be derived from the loaded tag corpus and item metadata, while the
   backend remains the source of truth for filtered item results.

   Change: add a small discriminated state model, pure reducers/helpers for
   selected item and load outcomes, and pure filter serialization/display
   helpers for platform, tag, date, archive status, watched status, and text
   query. Do not add tag/notes editing actions.

   Verify: first confirm the state/filter helper tests are absent, then add and
   run:

   ```sh
   rg "createLibraryViewModel|library_filters_serialize_platform_tag_date_status_and_text" frontend/src
   cd frontend && pnpm exec vitest run src/libraryState.test.ts src/libraryFilters.test.ts
   ```

7. Build the authenticated library shell, feed, and detail view [depends on #6]

   File(s): `frontend/src/App.tsx`, `frontend/src/LibraryView.tsx`,
   `frontend/src/LibraryFeed.tsx`, `frontend/src/ItemDetail.tsx`,
   `frontend/src/StatusBadge.tsx`, `frontend/src/Thumbnail.tsx`,
   `frontend/src/App.test.tsx`, `frontend/src/index.css`

   Reference behavior: M6 is an authenticated product surface, not a marketing
   page. It should render a quiet, dense library workspace with grid/list
   browsing, item detail, source deep-link, archive status, watched state, tags,
   date added, and thumbnail/fallback media. Pending and failed enrichment must
   remain visible. The layout must be responsive and must not require a local
   dev server for verification.

   Change: replace the placeholder shell with an authenticated app gate and a
   pure-renderable library view. Split feed cards, detail panel, thumbnail, and
   status badge into small components so files/functions stay under CI limits.
   Use the API-mediated thumbnail method from the frontend client when
   `thumbnail_s3_key` exists, and render a stable fallback when it does not.

   Verify: first confirm the UI test contract is absent, then add and run:

   ```sh
   rg "LibraryView|renders_authenticated_library_feed_with_detail_and_status" frontend/src
   cd frontend && pnpm exec vitest run src/App.test.tsx
   ```

8. Wire filter controls and canonical copy-link actions [depends on #7]

   File(s): `frontend/src/FilterBar.tsx`, `frontend/src/itemActions.ts`,
   `frontend/src/itemActions.test.ts`, `frontend/src/App.test.tsx`,
   `frontend/src/index.css`

   Reference behavior: M6 filters are for viewing only; they must not change
   tags, notes, or capture data. Copy-link uses `copy_url`, which is canonical
   when normalization succeeded and falls back to the original URL. Source
   navigation opens the live canonical/original source; if the source is gone,
   the snapshot metadata still remains in the detail view.

   Change: add filter controls for platform, explicit tag, date range, archive
   status, watched status, and text query, wired to the API filter query. Add a
   copy action through a small testable clipboard boundary and source deep-link
   actions in item cards/detail. Keep buttons/icons accessible and avoid
   provider embed work.

   Verify: first confirm the filter/copy UI tests are absent, then add and run:

   ```sh
   rg "copyCanonicalLink|renders_filter_controls_for_platform_tag_date_archive_watch_and_text" frontend/src
   cd frontend && pnpm exec vitest run src/itemActions.test.ts src/App.test.tsx
   ```

9. Document the M6 web contract [depends on #8]

   File(s): `README.md`, `frontend/README.md`, `backend/api/README.md`,
   `docs/architecture.md`, `CHANGELOG.md`

   Reference behavior: docs should describe only the M6 behavior now present.
   Do not claim tag editing, notes editing, tag rename/merge, provider embeds,
   Android offline queueing, Terraform deployment, alarms, or production bucket
   provisioning are complete.

   Change: update docs to state that the web UI authenticates with Cognito,
   browses saved items, shows archive status and API-mediated thumbnails when
   snapshots exist, supports filters/search, opens the source link, and copies
   canonical `copy_url`. Document the new list query parameters and thumbnail
   route in the API README.

   Verify: confirm docs mention the M6 contract and do not claim future phases
   are done:

   ```sh
   rg "web library|filter|copy_url|archive_status|thumbnail" README.md frontend/README.md backend/api/README.md docs/architecture.md CHANGELOG.md
   ! rg "tag rename is implemented|notes editing is implemented|Terraform snapshot bucket is deployed|provider embeds are implemented|Android offline queue" README.md frontend/README.md backend/api/README.md docs/architecture.md CHANGELOG.md
   ```

## Exit Gate

Run the canonical repository gate from the repository root:

```sh
make ci
```

The phase is complete only when `make ci` is green and the M6-specific tests
demonstrate:

- backend list filters are user-scoped and cover platform, tag, date, archive
  status, watched status, and title/notes text search,
- thumbnail reads are authenticated, user-scoped, and return only
  Linkdrop-owned snapshot bytes,
- the frontend API client attaches auth, serializes filters, retries once on
  `401`, and fetches thumbnails without hotlinking,
- the authenticated web UI renders feed/detail states for populated, empty,
  pending, failed, and error conditions,
- filter controls and canonical copy-link use the existing API contracts,
- no tag/notes editing, tag rename/merge, provider embeds, or Terraform deploy
  claims are introduced.
