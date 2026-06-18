# Linkdrop M4 Phase Plan - URL Normalization and Deduplication

## Scope

M4 makes captured URLs canonical at the persistence and API boundary. It does not implement enrichment, thumbnail archival, async processing workers, web library UI, Android UI changes, tag rename/merge, or Terraform changes.

Reference behavior comes from `LINKDROP-PLAN.md`, `docs/adr/0001-ahara-platform-topology.md`, and `docs/adr/0002-capture-first-async-enrichment.md`:

- Capture remains immediate and accepts zero mandatory fields beyond the shared URL.
- Invalid non-HTTP(S) URLs are rejected by the existing submitted URL validation.
- Original submitted URL and canonical normalized URL are both retained.
- Tracking parameters are stripped from canonical URLs while non-tracking query parameters are preserved.
- Known share-shortened hosts are normalized: `youtu.be` is converted without network access, and `vt.tiktok.com`/TikTok short hosts are resolved through a bounded resolver boundary.
- Resolver/enrichment failure never drops a valid capture. The item is still saved with the original URL, failed normalization status, and a copy URL fallback.
- Duplicate captures after normalization return the existing item instead of creating another library item.
- Client copy behavior is exposed as a canonical `copy_url` field on item DTOs. Until the web UI copy button exists, this is the contract future clients use.

The M4 exit gate is `make ci` green, with duplicate normalized URLs never creating duplicate library items.

## Steps

1. Add pure URL normalization for tracking stripping and `youtu.be`

   File(s): `backend/shared/src/url_normalization.rs`, `backend/shared/src/lib.rs`, `backend/shared/Cargo.toml`

   Reference behavior: submitted URLs are already validated as absolute HTTP(S) by `SubmittedUrl`; M4 adds canonicalization after validation. Canonical URLs strip common tracking parameters such as `utm_*`, `fbclid`, `gclid`, `gbraid`, `wbraid`, `mc_cid`, `mc_eid`, `igshid`, `_hsenc`, and `_hsmi`, while preserving meaningful query parameters. `https://youtu.be/{id}` canonicalizes to the equivalent YouTube watch URL with `v={id}` and preserves non-tracking query parameters such as `t` or `list`.

   Change: introduce a shared normalization module with a small result type, for example `NormalizedUrl`, and a pure function for parse-and-clean behavior. Register the module through `backend/shared/src/lib.rs`. Keep the function independent of storage and API code so both in-memory and PostgreSQL capture paths reuse the same rules.

   Verify: first confirm the symbol/test contract is absent, then add and run:

   ```sh
   rg "pub struct NormalizedUrl|normalizes_youtube_short_links_and_strips_tracking" backend/shared/src
   cd backend && cargo test --workspace --lib url_normalization::tests::normalizes_youtube_short_links_and_strips_tracking
   ```

2. Add a bounded short-link resolver boundary [depends on #1]

   File(s): `backend/shared/src/url_normalization.rs`, `backend/shared/Cargo.toml`

   Reference behavior: `vt.tiktok.com` and other TikTok short-share hosts require redirect resolution, but unit and integration tests must not depend on live platform redirects. If resolution fails for a valid submitted URL, capture still persists and records failed normalization instead of rejecting the drop.

   Change: add a `ShortUrlResolver` trait and production HTTP implementation with bounded redirect behavior. Add a no-network/fake resolver path for tests. The normalizer should only invoke the resolver for known short-share hosts and should normalize the resolved final URL with the same tracking stripping rules. Resolver errors produce an outcome with no canonical URL, a failed normalization status, and an error string suitable for `item_urls.normalization_error`.

   Verify: first confirm the resolver contract/test is absent, then add and run:

   ```sh
   rg "pub trait ShortUrlResolver|resolves_known_tiktok_short_links_before_normalizing" backend/shared/src
   cd backend && cargo test --workspace --lib url_normalization::tests::resolves_known_tiktok_short_links_before_normalizing
   ```

3. Add the canonical copy URL DTO contract [depends on #1]

   File(s): `backend/shared/src/library.rs`, `backend/shared/src/library_tests.rs`

   Reference behavior: the feature spec says copy returns the cleaned canonical URL. The current API has no web copy endpoint yet, so M4 exposes copy behavior directly on returned item DTOs. When `canonical_url` exists, `copy_url` equals it. When canonicalization failed or is pending, `copy_url` falls back to `original_url` so a saved entry remains usable.

   Change: add `copy_url: String` to item summary/detail serialization and centralize its construction so in-memory, PostgreSQL, list, detail, and capture responses all follow the same fallback rule.

   Verify: first confirm the field/test is absent, then add and run:

   ```sh
   rg "copy_url|item_summary_copy_url_prefers_canonical_url" backend/shared/src
   cd backend && cargo test --workspace --lib library::tests::item_summary_copy_url_prefers_canonical_url
   ```

4. Apply normalization and deduplication in the in-memory service [depends on #1, #2, #3]

   File(s): `backend/shared/src/library.rs`, `backend/shared/src/library_tests.rs`

   Reference behavior: the in-memory service backs API tests and must match database semantics. A repeated capture whose normalized canonical URL matches an existing item returns that item with `created = false`. Explicit tags are applied only to a newly created item; deduped repeat capture surfaces the existing item and does not mutate its tags.

   Change: run the shared normalizer during `InMemoryLibraryService::capture_item`, persist `original_url`, `canonical_url`, normalization status/error, and `copy_url`, and detect duplicates by `(user_id, canonical_url)` before creating a new item. Keep `client_capture_id` retry behavior intact and higher priority than creating a new item.

   Verify: first confirm the behavior test is absent, then add and run:

   ```sh
   rg "in_memory_capture_deduplicates_by_normalized_url" backend/shared/src/library_tests.rs
   cd backend && cargo test --workspace --lib library::tests::in_memory_capture_deduplicates_by_normalized_url
   ```

5. Apply normalization and deduplication in PostgreSQL capture [depends on #1, #2, #3]

   File(s): `backend/shared/src/library_pg.rs`, `backend/shared/src/library_pg_capture_helpers.rs`, `backend/shared/tests/library_pg_capture.rs`

   Reference behavior: `db/migrations/001_initial_schema.sql` already created `item_urls.canonical_url`, `normalization_status`, `normalization_error`, and a per-user partial unique index on canonical URL. M4 must use that schema rather than adding a parallel dedup table. `client_capture_id` idempotency from M3 remains retry-safe; canonical dedup handles distinct capture attempts that point to the same normalized source.

   Change: run the shared normalizer before inserting `item_urls`. If `client_capture_id` already exists for the user, return that item as before. Otherwise, if a successful canonical URL already exists for the user, return the existing item with `created = false`. For new items, insert the original URL, canonical URL, normalization status, and normalization error. Use the existing unique canonical index as a race-safety backstop and convert unique conflicts into fetching and returning the existing item.

   Verify: first confirm the database behavior test is absent, then add and run:

   ```sh
   rg "pg_capture_deduplicates_by_normalized_canonical_url" backend/shared/tests/library_pg_capture.rs
   cd backend && cargo test --workspace --test library_pg_capture pg_capture_deduplicates_by_normalized_canonical_url
   ```

6. Verify API capture responses expose canonical dedup and copy behavior [depends on #4, #5]

   File(s): `backend/api/tests/api_capture.rs`, `backend/api/src/item_routes.rs`

   Reference behavior: `POST /items` already returns `201 Created` for a new capture and `200 OK` when an existing item is surfaced. M4 extends that existing route contract: normalized repeat captures return the existing item with `created = false`, `canonical_url` populated when normalization succeeded, and `copy_url` set to the canonical URL.

   Change: add API tests around the existing capture route. Only adjust route code if the shared DTO field is not already serialized through the existing response path.

   Verify: first confirm the route behavior test is absent, then add and run:

   ```sh
   rg "capture_route_returns_existing_item_for_normalized_repeat" backend/api/tests/api_capture.rs
   cd backend && cargo test -p api --test api_capture capture_route_returns_existing_item_for_normalized_repeat
   ```

7. Document the M4 URL contract [depends on #6]

   File(s): `README.md`, `backend/README.md`, `backend/api/README.md`, `docs/architecture.md`, `CHANGELOG.md`

   Reference behavior: project docs should describe the behavior now present after M4 without claiming later phases are implemented. The docs should state that original URLs are retained, canonical URLs power deduplication and copy behavior, short-link resolution is best effort, and failed normalization does not block capture.

   Change: update the existing documentation surfaces that describe capture and item URLs. Do not add docs for enrichment, thumbnail archival, web copy buttons, or Android UI behavior beyond what exists.

   Verify: confirm the docs mention the M4 contract and do not claim future phases are done:

   ```sh
   rg "canonical URL|copy_url|youtu.be|vt.tiktok.com|deduplic" README.md backend/README.md backend/api/README.md docs/architecture.md CHANGELOG.md
   ! rg "thumbnail archival is implemented|web copy button|tag rename is implemented" README.md backend/README.md backend/api/README.md docs/architecture.md CHANGELOG.md
   ```

## Exit Gate

Run the canonical repository gate from the repository root:

```sh
make ci
```

The phase is complete only when `make ci` is green and the M4-specific tests demonstrate:

- tracking parameters are stripped while non-tracking query parameters are preserved,
- `youtu.be` and resolved TikTok short links canonicalize correctly,
- failed short-link resolution still saves the item,
- repeat captures by normalized canonical URL return the existing item,
- API capture responses expose canonical `copy_url` behavior.
