# Linkdrop M7 Phase Plan - Tag, Notes, and Inbox Management

## Scope

M7 completes the web organization workflows on top of the M6 viewing surface. It
adds an explicit inbox/organized state, item organization mutations for notes,
tags, watched status, and inbox status, plus tag rename/merge workflows for
cleaning up typo tags without losing item associations or usage counts.

M7 does not add Android organization UI, capture-time required fields, provider
embeds, saved searches, bulk edit workflows, export, Terraform deployment
wiring, alarms, or offline queueing. Android capture remains quick-drop only;
new captures still save immediately and land unsorted by default.

Reference behavior comes from `LINKDROP-PLAN.md`,
`docs/adr/0001-ahara-platform-topology.md`,
`docs/adr/0002-capture-first-async-enrichment.md`,
`docs/adr/0003-project-owned-snapshot-storage.md`,
`../ahara-standards/standards/typescript.md`,
`../ahara-standards/standards/testing.md`, and the M6 web/API contracts:

- Capture remains write-first with zero mandatory metadata fields.
- Tags are created only from explicit user input, never inferred or generated.
- New drops land with `inbox_status = unsorted`; leaving the inbox is an
  explicit later organization action.
- Notes are free-text item metadata and must not affect URL normalization,
  capture, deduplication, or archive status.
- Tag edits replace explicit item-tag associations; missing tag fields preserve
  existing tags, and an empty tag list clears tags.
- Tag rename changes display text only when it does not collide with another
  normalized tag; tag merge is the collision-safe typo cleanup path.
- Tag merge moves source item associations onto the target tag with
  `ON CONFLICT DO NOTHING`, deletes the source tag, and preserves correct usage
  counts through the existing database triggers.
- All item/tag mutations are scoped to the authenticated user.
- The frontend uses the existing authenticated API client boundary; no direct
  `fetch` calls are added.
- Rust and TypeScript files stay under 400 lines, functions under 75 lines, and
  cognitive complexity under 10.

The M7 exit gate is `make ci` green; the web library supports ongoing
organization through notes, explicit tags, watched status, inbox status, tag
rename, and tag merge without adding any capture-time tax.

## Steps

1. Add the inbox status schema and shared list contract

   File(s): `db/migrations/003_item_inbox_status.sql`,
   `db/migrations/rollback/003_item_inbox_status.sql`,
   `backend/shared/src/db.rs`, `backend/shared/src/domain.rs`,
   `backend/shared/src/library.rs`, `backend/shared/src/library_query.rs`,
   `backend/shared/src/library_in_memory.rs`,
   `backend/shared/src/library_tests.rs`,
   `backend/shared/tests/linkdrop_inbox_status.rs`

   Reference behavior: the inbox model is explicit and user-visible. New items
   default to `unsorted`; `organized` means the user deliberately moved the item
   out of the inbox. Inbox filtering is read-only and must not mutate notes,
   tags, watched status, capture data, or archive data.

   Change: add `items.inbox_status TEXT NOT NULL DEFAULT 'unsorted'` with a
   check constraint for `unsorted` and `organized`, plus rollback SQL. Add an
   `InboxStatus` enum following the existing `WatchStatus` pattern, expose it on
   `LibraryItemSummary`, and add optional `inbox_status` to `ListItemsQuery`.
   Update in-memory capture defaults and list filtering. Add migration constants
   and focused tests for the database default/check constraint, domain enum
   values, and in-memory inbox filtering.

   Verify: first confirm the inbox status contract/tests are absent, then add
   and run:

   ```sh
   rg "InboxStatus|linkdrop_model_adds_unsorted_inbox_status|in_memory_list_filters_items_by_inbox_status" backend/shared db/migrations
   cd backend && cargo test --workspace --lib domain::tests::domain_status_values_match_database_contract library::tests::in_memory_list_filters_items_by_inbox_status
   cd backend && cargo test --workspace --test linkdrop_inbox_status linkdrop_model_adds_unsorted_inbox_status
   ```

2. Apply inbox status in PostgreSQL item reads and the HTTP list route [depends on #1]

   File(s): `backend/shared/src/library_pg_sql.rs`,
   `backend/shared/src/library_pg_rows.rs`,
   `backend/shared/src/library_pg.rs`,
   `backend/shared/tests/library_pg_items.rs`,
   `backend/api/src/item_routes.rs`, `backend/api/tests/api_items.rs`,
   `backend/api/tests/api_capture.rs`

   Reference behavior: PostgreSQL item reads remain user-scoped. The API
   includes `inbox_status` on item summaries/details and accepts
   `GET /items?inbox_status=unsorted|organized`. Invalid enum values use the
   existing safe validation error shape. Capture responses surface new items as
   unsorted.

   Change: select `items.inbox_status`, parse it into `InboxStatus`, include it
   in item DTOs, and apply an optional inbox filter in `PgLibraryService`.
   Extend `ListItemsParams` to parse `inbox_status`. Update API tests and
   PostgreSQL tests for response shape, capture default, invalid query values,
   and user-scoped inbox filtering.

   Verify: first confirm the PostgreSQL/API inbox tests are absent, then add and
   run:

   ```sh
   rg "pg_list_items_filters_by_inbox_status|items_route_filters_by_inbox_status|capture_route_returns_unsorted_item" backend/shared/tests backend/api/tests
   cd backend && cargo test --workspace --test library_pg_items pg_list_items_filters_by_inbox_status
   cd backend && cargo test -p api --test api_items items_route_filters_by_inbox_status
   cd backend && cargo test -p api --test api_capture capture_route_returns_unsorted_item
   ```

3. Add the shared item organization mutation contract [depends on #1]

   File(s): `backend/shared/src/library.rs`,
   `backend/shared/src/library_in_memory.rs`,
   `backend/shared/src/library_tests.rs`

   Reference behavior: item organization is a deliberate authenticated update
   after capture. Missing fields preserve existing values. `tags: []` clears an
   item's explicit tags. Provided tags are trimmed, deduplicated by normalized
   name, and are the only source of new tag corpus entries. Notes may be set to
   an empty string. Watch and inbox status updates are independent.

   Change: evolve `UpdateItemRequest` into the item organization request with
   optional `watch_status`, `inbox_status`, `notes`, and `tags` fields, and
   rename the service method to `update_item`. Preserve existing watched-status
   behavior through the new request. Implement in-memory replacement of item
   tags, notes, watched status, and inbox status, including corpus usage counts
   and ranking after tag edits.

   Verify: first confirm the organization mutation test is absent, then add and
   run:

   ```sh
   rg "update_item_edits_tags_notes_watch_and_inbox|in_memory_update_item_replaces_tags_and_updates_corpus" backend/shared/src
   cd backend && cargo test --workspace --lib library::tests::in_memory_update_item_replaces_tags_and_updates_corpus
   cd backend && cargo test --workspace --lib library::tests::in_memory_update_item_edits_tags_notes_watch_and_inbox
   ```

4. Apply item organization mutations in PostgreSQL and the HTTP route [depends on #3]

   File(s): `backend/shared/src/library_pg_sql.rs`,
   `backend/shared/src/library_pg.rs`,
   `backend/shared/tests/library_pg_items.rs`,
   `backend/api/src/item_routes.rs`,
   `backend/api/tests/api_item_mutations.rs`

   Reference behavior: `PATCH /items/{item_id}` remains authenticated and
   user-scoped. It accepts any non-empty subset of `watch_status`,
   `inbox_status`, `notes`, and `tags`; missing fields preserve existing data.
   Item tag replacement is transactional with status/notes updates. Tag usage
   counts come from existing triggers, and the tag corpus returned by
   `GET /tags` should not include zero-use orphan tags.

   Change: update the PostgreSQL service to perform item organization changes
   in a transaction: validate ownership, update item statuses, upsert notes,
   and replace item tags when tags are provided. Update `TAG_CORPUS` to return
   active tags ordered by usage count then normalized name. Extend the existing
   `PATCH /items/{item_id}` route to use the new request and validation.

   Verify: first confirm the PostgreSQL/API mutation tests are absent, then add
   and run:

   ```sh
   rg "pg_update_item_replaces_tags_notes_watch_and_inbox|patch_item_route_edits_tags_notes_watch_and_inbox|patch_item_route_rejects_empty_organization_update" backend/shared/tests backend/api/tests
   cd backend && cargo test --workspace --test library_pg_items pg_update_item_replaces_tags_notes_watch_and_inbox
   cd backend && cargo test -p api --test api_item_mutations patch_item_route_edits_tags_notes_watch_and_inbox
   cd backend && cargo test -p api --test api_item_mutations patch_item_route_rejects_empty_organization_update
   ```

5. Add the shared tag rename and merge contract [depends on #4]

   File(s): `backend/shared/src/library.rs`,
   `backend/shared/src/library_in_memory.rs`,
   `backend/shared/src/library_tests.rs`

   Reference behavior: tag cleanup is explicit user action. Rename changes a
   tag's display name only when the normalized destination does not collide with
   another tag. Merge is the collision-safe path: source associations move to
   the target, duplicate source/target edges collapse, the source tag is
   removed, and the target's usage count reflects distinct associated items.

   Change: add `RenameTagRequest` and `MergeTagsRequest`, plus
   `rename_tag`/`merge_tags` methods on `LibraryService`. Implement in-memory
   rename and merge with user scoping, validation for empty names, collision
   handling, self-merge rejection, association preservation, and corpus
   reranking.

   Verify: first confirm the shared tag cleanup tests are absent, then add and
   run:

   ```sh
   rg "rename_tag_rejects_collision|merge_tags_moves_associations_and_reranks_corpus" backend/shared/src
   cd backend && cargo test --workspace --lib library::tests::in_memory_rename_tag_rejects_collision
   cd backend && cargo test --workspace --lib library::tests::in_memory_merge_tags_moves_associations_and_reranks_corpus
   ```

6. Apply tag rename and merge in PostgreSQL and the HTTP routes [depends on #5]

   File(s): `backend/shared/src/library_pg_sql.rs`,
   `backend/shared/src/library_pg.rs`,
   `backend/shared/tests/library_pg_tags.rs`,
   `backend/api/src/tag_routes.rs`, `backend/api/tests/api_tags.rs`

   Reference behavior: tag cleanup routes are authenticated and can only affect
   the current user's tags. Rename returns a validation error for normalized
   collisions; merge handles collisions by moving source edges to the target
   with `ON CONFLICT DO NOTHING`, deleting the source tag, and returning the
   updated tag corpus. Source and target must be different tags owned by the
   same user.

   Change: add PostgreSQL transactions for rename and merge. Extend tag routes:
   `PATCH /tags/{tag_id}` with `{ "display_name": "..." }` and
   `POST /tags/{source_tag_id}/merge` with `{ "target_tag_id": "..." }`.
   Return the updated corpus from both routes so clients can refresh chip
   ranking without guessing.

   Verify: first confirm the PostgreSQL/API tag cleanup tests are absent, then
   add and run:

   ```sh
   rg "pg_merge_tags_preserves_associations_and_usage_counts|tag_route_renames_tag|tag_route_merges_tags" backend/shared/tests backend/api/tests
   cd backend && cargo test --workspace --test library_pg_tags pg_merge_tags_preserves_associations_and_usage_counts
   cd backend && cargo test -p api --test api_tags tag_route_renames_tag
   cd backend && cargo test -p api --test api_tags tag_route_merges_tags
   ```

7. Extend frontend types, API client, filters, and state helpers [depends on #6]

   File(s): `frontend/src/types.ts`, `frontend/src/api.ts`,
   `frontend/src/api.test.ts`, `frontend/src/libraryFilters.ts`,
   `frontend/src/libraryFilters.test.ts`, `frontend/src/libraryState.ts`,
   `frontend/src/libraryState.test.ts`

   Reference behavior: frontend HTTP remains behind `ApiClient`. DTOs mirror
   the Rust API, including `inbox_status`. Filter serialization uses
   `inbox_status`. Item organization updates and tag cleanup actions are typed
   API calls; components should not build raw request paths.

   Change: add `InboxStatus`, include `inbox_status` on item summaries, extend
   list filters with `inboxStatus`, and add `updateItem`, `renameTag`, and
   `mergeTags` API methods. Add pure state helpers for replacing the selected
   detail, applying an updated item summary, and refreshing the tag corpus after
   rename/merge.

   Verify: first confirm the frontend API/state tests are absent, then add and
   run:

   ```sh
   rg "api_client_updates_item_organization|api_client_serializes_inbox_filter|library_state_replaces_selected_detail_after_organization" frontend/src
   cd frontend && pnpm exec vitest run src/api.test.ts src/libraryFilters.test.ts src/libraryState.test.ts
   ```

8. Build the item organization UI [depends on #7]

   File(s): `frontend/src/App.tsx`, `frontend/src/LibraryView.tsx`,
   `frontend/src/ItemDetail.tsx`, `frontend/src/ItemOrganizer.tsx`,
   `frontend/src/TagEditor.tsx`, `frontend/src/App.test.tsx`,
   `frontend/src/index.css`

   Reference behavior: item organization happens after capture inside the
   authenticated web library. The detail panel must support notes editing,
   explicit tag replacement, watched/unwatched transitions, and
   unsorted/organized transitions. It must not add capture UI, auto-generate
   tags, infer tags from metadata, or require any field before saving.

   Change: load item detail when a feed card is selected, render a compact
   organizer in the detail panel, and submit updates through `ApiClient`.
   Provide controls for notes, existing tag chips, free-text tag entry, watched
   status, and inbox status. Refresh the selected detail, visible list item, and
   tag corpus after a successful update. Split controls into small components
   so TypeScript file/function limits remain green.

   Verify: first confirm the item organizer UI tests are absent, then add and
   run:

   ```sh
   rg "ItemOrganizer|renders_item_organizer_for_notes_tags_watch_and_inbox|submits_item_organization_update" frontend/src
   cd frontend && pnpm exec vitest run src/App.test.tsx
   ```

9. Build the tag management UI [depends on #8]

   File(s): `frontend/src/TagManager.tsx`, `frontend/src/LibraryView.tsx`,
   `frontend/src/App.tsx`, `frontend/src/App.test.tsx`,
   `frontend/src/index.css`

   Reference behavior: tag rename/merge is for corpus cleanup, especially typo
   cleanup. It operates on explicit tags only and must not mutate item notes,
   URLs, watched status, archive status, or capture history. Merge should be a
   clear deliberate action because it removes the source tag.

   Change: add a compact tag management panel using the loaded tag corpus.
   Provide rename controls and merge controls that call the new API methods,
   then refresh the tag corpus, item list, and selected detail. Render empty
   corpus state without starter tags.

   Verify: first confirm the tag manager UI tests are absent, then add and run:

   ```sh
   rg "TagManager|renders_tag_manager_for_rename_and_merge|renders_empty_tag_corpus_without_starter_tags" frontend/src
   cd frontend && pnpm exec vitest run src/App.test.tsx
   ```

10. Document the M7 organization contract [depends on #9]

   File(s): `README.md`, `frontend/README.md`, `backend/api/README.md`,
   `docs/architecture.md`, `CHANGELOG.md`

   Reference behavior: docs should describe only the M7 behavior now present.
   Do not claim Android organization UI, bulk edit, saved searches, export,
   provider embeds, Terraform deployment, alarms, or offline queueing are done.

   Change: document `inbox_status`, item organization mutation semantics, tag
   rename/merge routes, and the frontend organization surface. Clarify that
   tags still come only from explicit user input and new captures still land
   unsorted with zero required metadata fields.

   Verify: confirm docs mention the M7 contract and do not claim future phases:

   ```sh
   rg "inbox_status|tag rename|tag merge|notes editing|organization" README.md frontend/README.md backend/api/README.md docs/architecture.md CHANGELOG.md
   ! rg "Android organization UI is implemented|bulk edit is implemented|saved searches are implemented|export is implemented|Terraform deployment is complete|provider embeds are implemented|offline queueing is implemented" README.md frontend/README.md backend/api/README.md docs/architecture.md CHANGELOG.md
   ```

## Exit Gate

Run the canonical repository gate from the repository root:

```sh
make ci
```

The phase is complete only when `make ci` is green and the M7-specific tests
demonstrate:

- new captures default to `inbox_status = unsorted`,
- item reads and list filters include inbox status,
- item organization updates are authenticated, user-scoped, partial, and
  transactional in PostgreSQL,
- notes editing preserves free text and remains searchable through the existing
  list search,
- explicit item tag replacement updates usage counts and chip ranking,
- tag rename rejects normalized collisions,
- tag merge preserves item associations, collapses duplicate edges, removes the
  source tag, and reranks the corpus,
- the frontend uses only the typed API client for organization mutations,
- no capture-time required fields, auto-generated tags, Android organization
  UI, provider embeds, bulk edit workflows, or deployment claims are introduced.
