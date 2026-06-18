# M2 — Authenticated API Foundation Phase Plan

This expands only `M2 — Authenticated API Foundation` from [LINKDROP-PLAN.md](LINKDROP-PLAN.md). M2 creates the authenticated Rust HTTP API foundation over the M1 data model. It does not implement Android capture, URL normalization behavior, async enrichment, thumbnail storage, web UI workflows, tag rename/merge workflows, or Terraform API resources.

## Reference Context

- Plan context/reuse map: [LINKDROP-PLAN.md](LINKDROP-PLAN.md)
- Current architecture and M1 schema/domain contracts: [docs/architecture.md](docs/architecture.md), [LINKDROP-M1-PHASE.md](LINKDROP-M1-PHASE.md)
- Ahara API/auth/error reference: `../ahara-business/backend/api/src/lib.rs`, `../ahara-business/backend/api/src/main.rs`, `../ahara-business/backend/api/src/cors.rs`
- Ahara shared auth/config/error reference: `../ahara-business/backend/shared/src/auth.rs`, `config.rs`, `error.rs`, `db.rs`
- Ahara platform database/API contract: `../ahara/INTEGRATION.md` Steps 4 and 5
- Standards: `../ahara-standards/standards/rust.md`, `testing.md`, `project-structure.md`

## Steps

1. Add shared runtime config, public error mapping, and database connection helpers.
   - File(s): `backend/Cargo.toml`, `backend/shared/Cargo.toml`, `backend/shared/src/config.rs`, `backend/shared/src/error.rs`, `backend/shared/src/db.rs`, `backend/shared/src/lib.rs`
   - Reference behavior: Ahara Business keeps environment parsing and public error mapping in `shared`, maps internal/database/external errors to safe public responses, builds PostgreSQL URLs with `sslmode=require`, and keeps database credentials in env/SSM-fed config rather than migrations. M1 already owns migration constants in `shared::db`; M2 extends that module without removing those constants.
   - Change: add workspace/shared dependencies needed for config and database access (`sqlx`, `tokio` if not already present); add `AppConfig`, `DatabaseConfig`, `ApiConfig`, and `CognitoConfig` with `from_env`/`from_lookup`; add `AppError`, `AppResult`, and `PublicError`; extend `shared::db` with `DbPool`, `database_url`, and `connect_pool`; keep M1 migration constants/tests intact.
   - Verify: `rg "pub struct AppConfig" backend/shared/src/config.rs && rg "pub enum AppError" backend/shared/src/error.rs && rg "pub fn database_url" backend/shared/src/db.rs && cd backend && cargo test --workspace --lib config::tests && cargo test --workspace --lib error::tests && cargo test --workspace --lib db::tests`. Red before: `config.rs`, `error.rs`, and database connection helpers do not exist.

2. Add the shared Cognito auth verifier and user context. [depends on #1]
   - File(s): `backend/Cargo.toml`, `backend/shared/Cargo.toml`, `backend/shared/src/auth.rs`, `backend/shared/src/lib.rs`
   - Reference behavior: Every backend route except health uses shared Cognito authentication and testable in-process token validation. Ahara Business models this as an `AuthVerifier` trait, production `CognitoJwtVerifier`, `UserContext`, bearer extraction, JWKS fetching, and test-only unverified claim decoding for router tests.
   - Change: add `UserContext`, `AuthVerifier`, `JwksProvider`, `CognitoJwtVerifier`, HTTP JWKS provider, `extract_bearer`, and `decode_unverified_claims`. Validate signed Cognito access tokens against issuer/client ID in production; keep unverified decoding available only as a test utility for API tests.
   - Verify: `rg "pub trait AuthVerifier" backend/shared/src/auth.rs && rg "pub struct CognitoJwtVerifier" backend/shared/src/auth.rs && cd backend && cargo test --workspace --lib auth::tests`. Red before: `shared::auth` does not exist.

3. Add the Linkdrop library service boundary and in-memory implementation. [depends on #1]
   - File(s): `backend/shared/src/library.rs`, `backend/shared/src/lib.rs`, `backend/shared/Cargo.toml`
   - Reference behavior: M2 requires trait-based service boundaries and API tests that do not require real external services. The service boundary should expose M1 concepts only: authenticated user ownership, item list/detail reads, tag corpus reads, and a basic existing-item mutation. Capture creation belongs to M3; URL normalization belongs to M4; notes/tag editing/inbox organization belong to M7.
   - Change: add `LibraryService` with methods for `list_items`, `get_item`, `list_tag_corpus`, and `update_item_watch_status`; add request/response DTOs for `LibraryItemSummary`, `LibraryItemDetail`, `TagCorpusEntry`, and `UpdateItemRequest`; add an `InMemoryLibraryService` for API tests. Limit the M2 mutation to `watch_status` on an existing item.
   - Verify: `rg "pub trait LibraryService" backend/shared/src/library.rs && rg "pub struct InMemoryLibraryService" backend/shared/src/library.rs && cd backend && cargo test --workspace --lib library::tests`. Red before: `shared::library` does not exist.

4. Add the PostgreSQL library service over the M1 schema. [depends on #3]
   - File(s): `backend/shared/src/library_pg.rs`, `backend/shared/src/library.rs`, `backend/shared/src/lib.rs`
   - Reference behavior: Production API state must be deployable against Ahara's shared PostgreSQL database using the M1 tables and authenticated user ownership. Unknown users should see empty list/tag corpus results; item detail and mutation must be scoped by Cognito `sub` and return not found for another user's item.
   - Change: add `PgLibraryService` backed by `DbPool`; implement item list/detail reads from `items`, `item_urls`, `metadata_snapshots`, `item_notes`, and tag joins; implement tag corpus reads from `tags` and `tag_usage_counts` sorted by usage count descending then normalized name; implement `watch_status` update on existing items only. Do not add capture/notes/tags mutation behavior.
   - Verify: `rg "pub struct PgLibraryService" backend/shared/src/library_pg.rs && rg "impl LibraryService for PgLibraryService" backend/shared/src/library_pg.rs && cd backend && cargo test --workspace --lib library_pg::tests`. Red before: `PgLibraryService` does not exist.

5. Add the API crate shell with health, auth context, CORS, structured errors, and Lambda entrypoint. [depends on #2] [depends on #3] [depends on #4]
   - File(s): `backend/Cargo.toml`, `backend/api/Cargo.toml`, `backend/api/src/main.rs`, `backend/api/src/lib.rs`, `backend/api/src/cors.rs`, `backend/api/tests/support/mod.rs`, `backend/api/tests/api_foundation.rs`, `backend/api/README.md`, `platform.yml`
   - Reference behavior: Ahara APIs use a thin Lambda `main`, an Axum router in a library crate, `CorsLayer` for non-preflight responses, `ApiError` wrapping `AppError::public_error`, a test verifier that decodes unverified claims, and split API tests. Since M2 adds a deployable API Lambda, `platform.yml` must declare the `api` lambda artifact instead of `rust_artifacts: {}`.
   - Change: register `backend/api` in the workspace; add API dependencies (`axum`, `lambda_http`, `tower`, `tower-http`, `serde_json`, `base64`, `tracing`, `tracing-subscriber` as needed); add `ApiState::from_env` using `AppConfig`, `connect_pool`, `CognitoJwtVerifier`, and `PgLibraryService`; add `/health` without auth and `/me` with auth; add CORS; add structured error responses. Keep `backend/api/tests/support/mod.rs` to primitives used by every API integration test to avoid dead-code warnings under `make ci`.
   - Verify: `rg "\"api\"" backend/Cargo.toml && rg "lambdas:" platform.yml && rg "fn health_route_returns_service_status_without_auth" backend/api/tests/api_foundation.rs && cd backend && cargo test -p api --test api_foundation`. Red before: the API crate, API test target, and lambda artifact declaration do not exist.

6. Add authenticated item list and detail routes. [depends on #5]
   - File(s): `backend/api/src/item_routes.rs`, `backend/api/src/lib.rs`, `backend/api/tests/api_items.rs`, `backend/api/tests/support/mod.rs`
   - Reference behavior: M2 requires item listing, item detail, API auth tests, empty library behavior, and basic item reads. Routes must require Cognito auth, use the `LibraryService` trait, return safe structured errors, and must not implement capture, normalization, notes editing, or tag editing.
   - Change: add `GET /items` and `GET /items/{item_id}`; wire them through `LibraryService`; add API tests for missing auth, empty library `[]`, seeded item list/detail reads, and not-found item detail shape.
   - Verify: `rg "fn items_route_returns_empty_library" backend/api/tests/api_items.rs && rg "fn items_route_returns_seeded_item_detail" backend/api/tests/api_items.rs && cd backend && cargo test -p api --test api_items`. Red before: item route tests do not exist.

7. Add the authenticated tag corpus route and empty-state API test. [depends on #5]
   - File(s): `backend/api/src/tag_routes.rs`, `backend/api/src/lib.rs`, `backend/api/tests/api_tags.rs`, `backend/api/tests/support/mod.rs`
   - Reference behavior: The tag corpus starts empty and contains only explicitly applied tags. M2 exposes the reusable corpus endpoint for clients; M3/M7 own chip application and tag management workflows.
   - Change: add `GET /tags`; wire it through `LibraryService::list_tag_corpus`; add tests for missing auth and empty corpus `[]`. If seeded tags are used in test support, keep ranking behavior aligned with usage count descending then normalized name ascending.
   - Verify: `rg "fn tags_route_returns_empty_corpus" backend/api/tests/api_tags.rs && cd backend && cargo test -p api --test api_tags`. Red before: tag route tests do not exist.

8. Add the basic existing-item watch-status mutation endpoint. [depends on #5] [depends on #6]
   - File(s): `backend/api/src/item_routes.rs`, `backend/api/tests/api_item_mutations.rs`, `backend/api/tests/support/mod.rs`
   - Reference behavior: M2 includes basic item mutation endpoints, while later phases own capture creation, tag editing, notes editing, inbox organization, and web workflow completion. The M2 mutation is limited to the existing M1 watched/unwatched state so it does not start M3 or M7.
   - Change: add `PATCH /items/{item_id}` accepting `UpdateItemRequest { watch_status }`; require auth; return the updated item detail; map service validation/not-found errors through structured `ApiError`. Do not add request fields for tags, notes, canonical URL, capture, or archive metadata.
   - Verify: `rg "fn patch_item_route_updates_watch_status" backend/api/tests/api_item_mutations.rs && cd backend && cargo test -p api --test api_item_mutations`. Red before: mutation route tests do not exist.

9. Refresh M2 API documentation.
   - File(s): `backend/README.md`, `backend/api/README.md`, `docs/architecture.md`, `docs/development.md`
   - Reference behavior: Repo docs describe current-state contracts. After M2, `api` is a registered buildable Lambda crate, `shared` owns config/auth/error/library service boundaries, and public API behavior covers health, auth context, item reads, tag corpus reads, and watch-status mutation only.
   - Change: update docs to describe the M2 API crate, route surface, auth/error/CORS contract, and make-ci expectations. Do not claim Android capture, URL normalization, enrichment, web library UI, tag management, notes editing, or Terraform API resources are complete.
   - Verify: `rg "GET /health|GET /me|GET /items|GET /tags|PATCH /items" backend/README.md backend/api/README.md docs/architecture.md && ! rg "Android capture is implemented|URL normalization is implemented|enrichment is implemented|web library UI is implemented|Terraform API resources are complete" backend docs README.md`. Red before: docs do not describe the M2 API route surface.

## Exit Gate

Run after all steps:

```bash
make ci
```

The phase is complete when `make ci` is green, the `api` crate is registered and linted under the existing Rust complexity/file-size gates, and the authenticated API contract is covered by the split API tests.
