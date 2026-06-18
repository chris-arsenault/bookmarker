# M1 — Database and Domain Model Phase Plan

This expands only `M1 — Database and Domain Model` from [LINKDROP-PLAN.md](LINKDROP-PLAN.md). M1 creates the durable PostgreSQL shape and shared Rust domain boundaries for Linkdrop. It does not implement API routes, capture behavior, Android share handling, URL normalization logic, enrichment workers, Terraform resources, or UI behavior.

## Reference Context

- Plan context/reuse map: [LINKDROP-PLAN.md](LINKDROP-PLAN.md)
- Current architecture: [docs/architecture.md](docs/architecture.md)
- Capture-first processing decision: [docs/adr/0002-capture-first-async-enrichment.md](docs/adr/0002-capture-first-async-enrichment.md)
- Snapshot storage decision: [docs/adr/0003-project-owned-snapshot-storage.md](docs/adr/0003-project-owned-snapshot-storage.md)
- Ahara database contract: `../ahara/INTEGRATION.md` Step 5
- Standards: `../ahara-standards/standards/project-structure.md`, `rust.md`, `testing.md`
- Existing migration/test patterns: `../ahara-business/backend/shared/src/db.rs`, `../ahara-business/backend/shared/tests/mail_model.rs`

## Steps

1. Add the Linkdrop model migration, rollback, and migration constants.
   - File(s): `db/migrations/001_create_linkdrop_model.sql`, `db/migrations/rollback/001_create_linkdrop_model.sql`, `db/migrations/README.md`, `backend/shared/src/db.rs`, `backend/shared/src/lib.rs`
   - Reference behavior: Ahara migrations live under `db/migrations`, rollback files mirror forward files under `db/migrations/rollback`, filenames sort lexicographically, and migration SQL must contain only tables, indexes, constraints, triggers/functions, and data-shape DDL. Per [ADR 0002](docs/adr/0002-capture-first-async-enrichment.md), saved items must exist before normalization/enrichment succeeds. Per [ADR 0003](docs/adr/0003-project-owned-snapshot-storage.md), snapshot metadata persists in PostgreSQL while thumbnail binaries live in project-owned S3. Dedupe is per authenticated user because Linkdrop is a private library visible across that user's devices.
   - Change: add one forward migration defining `users`, `items`, `item_urls`, `tags`, `item_tags`, `tag_usage_counts`, `item_notes`, `metadata_snapshots`, and `processing_jobs`; add reverse-order rollback that drops triggers/functions and all M1 tables; add `backend/shared::db` constants using `include_str!` for forward and rollback SQL. The schema should include: `users.cognito_sub` unique; `items.user_id` ownership and watched/unwatched status; `item_urls.original_url`, nullable `canonical_url`, normalization status, and a partial unique dedupe index on `(user_id, canonical_url)` where canonical URL is present; generated per-user tag normalization with unique `(user_id, normalized_name)`; item-tag ownership FKs and explicit-only application source; tag usage count maintenance from item-tag edges; notes by item; metadata snapshot fields with pending/succeeded/failed archive status; processing jobs with kind/status checks, nonnegative attempts, idempotency key, and unique `(item_id, job_kind)`.
   - Verify: `bash -lc 'for table in users items item_urls tags item_tags tag_usage_counts item_notes metadata_snapshots processing_jobs; do rg "CREATE TABLE ${table}" db/migrations/001_create_linkdrop_model.sql >/dev/null; done' && test -f db/migrations/rollback/001_create_linkdrop_model.sql && ! rg "CREATE ROLE|CREATE USER|GRANT|REVOKE|ALTER DEFAULT PRIVILEGES|CREATE DATABASE" db/migrations && cd backend && cargo test --workspace --lib db::tests::migration_constants_reference_linkdrop_tables`. Red before: the migration, rollback, and `shared::db` constants do not exist.

2. Add the PostgreSQL migration round-trip integration test and wire integration tests into `make test`. [depends on #1]
   - File(s): `backend/shared/tests/support/mod.rs`, `backend/shared/tests/linkdrop_migration.rs`, `Makefile`
   - Reference behavior: Ahara Rust database tests exercise SQL against real PostgreSQL from Rust integration tests. The existing Ahara Business pattern includes migration SQL through `shared::db`, starts `postgres:16-alpine` for the test, applies forward SQL, applies rollback SQL, and verifies tables are created and removed. M1's exit requires migrations to apply and rollback cleanly in tests, so `make ci` must run integration tests, not only lib tests. The current `make ci` lint gate enforces Rust cognitive complexity, Rust function length, and Rust source files under 400 lines; M1 tests must stay split across focused files instead of growing one large integration file.
   - Change: add reusable PostgreSQL/Docker test helpers in `backend/shared/tests/support/mod.rs`; add `linkdrop_migration_round_trip_applies_and_rolls_back` in `backend/shared/tests/linkdrop_migration.rs`; update `Makefile` so the Rust test command runs `cargo test --workspace --all-targets` while preserving `RUST_CLIPPY_FLAGS`, `rust-lines-check`, and the frontend ESLint gate.
   - Verify: `rg "cargo test --workspace --all-targets" Makefile && rg "RUST_CLIPPY_FLAGS.*cognitive_complexity.*too_many_lines" Makefile && rg "rust-lines-check" Makefile && rg "fn linkdrop_migration_round_trip_applies_and_rolls_back" backend/shared/tests/linkdrop_migration.rs && cd backend && cargo test --workspace --test linkdrop_migration linkdrop_migration_round_trip_applies_and_rolls_back`. Red before: the integration test target and all-targets Makefile command do not exist.

3. Add database tests for ownership, constraints, and canonical dedupe keys. [depends on #2]
   - File(s): `backend/shared/tests/linkdrop_constraints.rs`
   - Reference behavior: The plan requires focused database tests for constraints and dedup keys. Capture is user-owned, stores both original and canonical URLs, and deduplicates repeat URLs after normalization by canonical URL. A pending item can exist before canonical URL is known, so dedupe must be a partial key over non-null canonical URLs.
   - Change: add `linkdrop_model_enforces_owned_items_and_canonical_dedup_keys`. The test should apply the migration, insert two users, verify duplicate `cognito_sub` fails, verify an item URL cannot point across owners, verify duplicate non-null canonical URLs fail for the same user, verify the same canonical URL is allowed for a different user, and verify multiple pending/null canonical URLs are allowed for the same user.
   - Verify: `rg "fn linkdrop_model_enforces_owned_items_and_canonical_dedup_keys" backend/shared/tests/linkdrop_constraints.rs && cd backend && cargo test --workspace --test linkdrop_constraints linkdrop_model_enforces_owned_items_and_canonical_dedup_keys`. Red before: the named test is absent.

4. Add database tests for explicit tag corpus counts and merge invariants. [depends on #2]
   - File(s): `backend/shared/tests/linkdrop_tags.rs`
   - Reference behavior: The confirmed tag corpus starts empty and contains only tags the user explicitly applies. The backend maintains usage counts for chip ranking. Tag rename/merge later must not allow typo variants or duplicate item-tag edges to corrupt counts.
   - Change: add `linkdrop_model_maintains_explicit_tag_usage_and_merge_invariants`. The test should apply the migration, create tags only through explicit item-tag edges, verify normalized tag-name uniqueness per user, verify duplicate item-tag edges fail or no-op through `ON CONFLICT DO NOTHING`, verify usage counts update from item-tag insert/delete behavior, and verify a source tag can be merged into a target tag without double-counting an item that already had the target tag.
   - Verify: `rg "fn linkdrop_model_maintains_explicit_tag_usage_and_merge_invariants" backend/shared/tests/linkdrop_tags.rs && cd backend && cargo test --workspace --test linkdrop_tags linkdrop_model_maintains_explicit_tag_usage_and_merge_invariants`. Red before: the named test is absent.

5. Add database tests for archive and processing status idempotency. [depends on #2]
   - File(s): `backend/shared/tests/linkdrop_processing.rs`
   - Reference behavior: ADR 0002 requires pending/failed/succeeded status to be visible and idempotent because enrichment may retry or fail after the item is saved. ADR 0003 requires snapshot metadata to remain durable independently of source availability. Processing jobs must be retry-safe and not duplicated for the same item/job kind.
   - Change: add `linkdrop_model_supports_idempotent_archive_and_processing_status_updates`. The test should apply the migration, create an item, upsert the same metadata snapshot status more than once and assert one row remains, reject invalid archive and processing statuses, upsert the same processing job more than once through the `(item_id, job_kind)` conflict path, assert one job row remains, and verify attempt counts cannot become negative.
   - Verify: `rg "fn linkdrop_model_supports_idempotent_archive_and_processing_status_updates" backend/shared/tests/linkdrop_processing.rs && cd backend && cargo test --workspace --test linkdrop_processing linkdrop_model_supports_idempotent_archive_and_processing_status_updates`. Red before: the named test is absent.

6. Add shared Rust status and record domain types.
   - File(s): `backend/Cargo.toml`, `backend/shared/Cargo.toml`, `backend/shared/src/domain.rs`, `backend/shared/src/lib.rs`
   - Reference behavior: Ahara Rust standards keep testable logic in `lib.rs`, share workspace dependencies from the workspace root, and avoid stringly-typed business state at service boundaries. M1's database status strings are the contract later API and processing phases will use.
   - Change: add workspace/shared dependencies needed for domain records (`serde`, `time`, `uuid`, `thiserror`, and `url` if used by step #7); add `shared::domain` with serializable enums for `ArchiveStatus`, `WatchStatus`, `ProcessingJobKind`, and `ProcessingStatus`, plus lightweight record structs for item URL, tag, item note, metadata snapshot, and processing job boundaries. Add string conversion helpers/tests that match the M1 SQL status values exactly.
   - Verify: `rg "pub enum ArchiveStatus" backend/shared/src/domain.rs && rg "pub enum ProcessingJobKind" backend/shared/src/domain.rs && cd backend && cargo test --workspace --lib domain::tests::domain_status_values_match_database_contract`. Red before: `shared::domain` and the named test do not exist.

7. Add shared Rust validation boundaries for URLs and explicit tags. [depends on #6]
   - File(s): `backend/shared/src/domain.rs`
   - Reference behavior: Capture has zero mandatory metadata fields, but it still accepts a link rather than arbitrary text. Tags are never inferred or auto-generated; explicit tag names must trim to a non-empty corpus key while preserving the user's display text. URL normalization belongs to M4, so M1 validation must not strip tracking params, resolve shorteners, or canonicalize.
   - Change: add `SubmittedUrl` validation that accepts absolute `http` and `https` URLs and preserves the submitted string; reject empty input, non-URLs, and non-web schemes. Add `TagName` validation that trims display text, rejects empty names, and exposes a lowercased normalized key without auto-generating any tag from a URL/title/platform.
   - Verify: `rg "pub struct SubmittedUrl" backend/shared/src/domain.rs && rg "pub struct TagName" backend/shared/src/domain.rs && cd backend && cargo test --workspace --lib domain::tests`. Red before: the validation types/tests do not exist.

8. Refresh M1 database/domain documentation.
   - File(s): `db/migrations/README.md`, `backend/README.md`, `docs/architecture.md`
   - Reference behavior: Repo docs describe current-state contracts, not implementation history. After M1, the database and shared crate have concrete responsibilities, while API routes, capture behavior, normalization, enrichment, and UI flows remain owned by later phases.
   - Change: update the migration README with the M1 migration/rollback contract and platform limitations; update backend docs to note that `shared` now owns DB constants and domain validation; update architecture's database section to reflect the M1 tables without claiming any later API/Android/UI behavior is implemented.
   - Verify: `rg "001_create_linkdrop_model.sql" db/migrations/README.md && rg "SubmittedUrl|TagName|ArchiveStatus" backend/README.md docs/architecture.md && ! rg "implemented capture endpoint|Android share target is implemented|normalization is implemented|enrichment is implemented" README.md docs backend`. Red before: the docs do not describe the concrete M1 schema/domain contracts.

## Exit Gate

Run after all steps:

```bash
make ci
```

The phase is complete when `make ci` is green and the M1 integration tests apply and roll back the Linkdrop migration against PostgreSQL.
