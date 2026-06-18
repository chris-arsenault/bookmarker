# Linkdrop — Implementation Plan

Linkdrop captures shared links from Android with no required fields, syncs them to the Ahara platform, enriches and snapshots metadata asynchronously, and exposes a private visual library on web and Android. This plan covers the backend, frontend, Android share target, database, archival, and platform integration; broad social-sharing integrations are out of scope beyond canonical copy-link.

## Confirmed decisions

- Project key, prefix, and database name are `linkdrop`.
- Primary hostnames are `linkdrop.ahara.io` and `api.linkdrop.ahara.io`.
- Linkdrop uses the shared Ahara ALB, VPC, PostgreSQL/RDS, Cognito pool, and Terraform state.
- Web and Android use the Linkdrop public Cognito app client registered as `linkdrop-app`.
- Android is a native client under `android/` and consumes the same authenticated API as web.
- The initial tag corpus starts empty; chips appear from explicit tag usage.
- Metadata enrichment starts with best-effort unauthenticated fetching and keeps extension points for official provider credentials.
- Thumbnail snapshots are stored in a Linkdrop-owned private S3 bucket, not hotlinked.
- Capture persists immediately and enrichment runs asynchronously.

## Context / Reuse Map

| Source | Reuse |
| ---- | ---- |
| `../ahara/INTEGRATION.md` | Platform integration contract, CI workflow, shared RDS/Cognito/ALB/state rules |
| `../ahara-standards/standards/` | Project structure, Rust, TypeScript, Terraform, testing, docs, and git conventions |
| `../ahara-tf-patterns/modules/` | `platform-context`, `alb-api`, `lambda`, `website`, and `cognito-app` Terraform modules |
| `../ahara-business` | Modern Rust API shape, in-process Cognito verifier, API error contract, frontend API client, Vitest pattern |
| `../tastebase` | Async processing Lambda pattern, media bucket permissions, enrichment-style workflow |
| `../ahara-access` | Private asset access reference; not used for thumbnail storage in the baseline |
| `../ahara-infra` | Project deployer registration, database registration, auth-trigger client map, app authorization table |

Gaps built new for Linkdrop:

- URL normalization and dedup domain logic for shared social/video links.
- Linkdrop item, tag corpus, archive snapshot, notes, and watched-state schema.
- Android share target and quick-drop UI.
- Web library UI tailored to visual link browsing and tag management.

## Cross-Cutting Constraints

- Every backend route except health uses shared Cognito authentication and testable in-process token validation.
- Capture accepts a URL with zero mandatory metadata fields.
- Enrichment, normalization, deduplication, and snapshotting are idempotent.
- Tags are stored only when explicitly applied by the user.
- The original submitted URL and canonical normalized URL are both retained.
- Source deletion leaves the snapshot record browsable.
- Client copy actions return the canonical normalized URL.
- `make ci` is the canonical local verification command for every implementation phase.
- Local development servers are started only when explicitly requested.

## Milestones

### M0 — Platform-Ready Scaffold

Establish the deployable repo skeleton without implementing product behavior.

- Add Rust, TypeScript, Terraform, database, Android, and documentation scaffolds according to Ahara standards.
- Add `platform.yml`, shared CI workflow, `Makefile`, `.node-version`, package manager pinning, Rust formatting/lint config, and local env examples.
- Register Linkdrop in `ahara-infra` control and migration services.
- Reserve a unique shared ALB listener priority range for Linkdrop.
- Exit: `make ci` green; the repo has standard Ahara scaffolding and no half-registered build members.

### M1 — Database and Domain Model

Create the durable data shape for captured links and tag organization.

- Add migrations for users, items, item URLs, tags, item-tag edges, tag usage counts, notes, watched state, metadata snapshots, processing jobs, and archive status.
- Add rollback migrations and focused database tests for constraints, dedup keys, tag merge invariants, and idempotent status updates.
- Define shared Rust domain types and validation boundaries.
- Exit: `make ci` green; migrations apply and rollback cleanly in tests.

### M2 — Authenticated API Foundation

Build the HTTP API surface and shared service boundaries.

- Implement health, auth context, item listing, item detail, tag corpus, and basic item mutation endpoints.
- Use the Ahara-style Cognito verifier, structured error responses, CORS layer, and trait-based service boundaries.
- Add API tests for auth, error shapes, empty library, tag corpus empty state, and basic item reads.
- Exit: `make ci` green; authenticated API contract is test-covered and deployable behind the shared ALB.

### M3 — Quick-Drop Capture and Android Share Target

Deliver the core Android capture loop.

- Implement capture endpoint that accepts URL plus optional explicit tags and persists immediately.
- Implement tag usage updates from explicit tags only.
- Add native Android share target with `ACTION_SEND` handling, immediate save, optional chips, free-text tags, and confirmation toast.
- Add Android API client authentication and retry-safe request handling.
- Exit: `make ci` green; a shared URL can be captured from Android with no mandatory fields and appears through the API.

### M4 — URL Normalization and Deduplication

Make stored and copied URLs canonical.

- Implement normalization for common tracking parameters and known short-share hosts such as `youtu.be` and `vt.tiktok.com`.
- Store original and canonical URLs, deduplicate by canonical URL, and surface existing items on repeat capture.
- Add tests for normalization, repeated capture, query preservation, and copy-link canonical behavior.
- Exit: `make ci` green; duplicate normalized URLs never create duplicate library items.

### M5 — Async Enrichment and Snapshot Archival

Populate visual metadata without blocking capture.

- Add the `processing` Lambda and invocation from capture and retry paths.
- Fetch title, thumbnail, author/channel, platform, and duration through best-effort provider/OpenGraph extractors.
- Copy thumbnails to the Linkdrop snapshot bucket and update pending/succeeded/failed archive status.
- Preserve graceful degradation for private, dead, blocked, and malformed sources.
- Exit: `make ci` green; processing is idempotent and failed enrichment leaves a saved item with visible status.

### M6 — Web Library Interface

Build the web viewing and organization surface.

- Implement authenticated React app shell, shared API client, visual grid/list feed, item detail, inline playback or source deep-link, copy-link action, and archive status display.
- Add filters for platform, tag, date, archive status, watched state, and free-text search over title and notes.
- Add empty states, error handling, and responsive layouts consistent with Ahara frontend standards.
- Exit: `make ci` green; the web UI supports browsing, filtering, item detail, and canonical copy-link.

### M7 — Tag, Notes, and Inbox Management

Complete the organization workflows.

- Add edit-tags-on-item, notes editing, watched/unwatched transitions, unsorted inbox state, and deliberate organize actions.
- Add tag rename and merge operations that preserve item associations and usage counts.
- Add tests for typo cleanup, merge collisions, tag chip ranking, and watched/inbox filters.
- Exit: `make ci` green; the library supports ongoing organization without capture-time friction.

### M8 — Deploy, Operations, and Hardening

Make Linkdrop production-ready on Ahara.

- Complete Terraform for API, processing Lambda, frontend, Cognito client, SSM auth-trigger client registration, snapshot bucket, and runtime config.
- Add local `scripts/deploy.sh`, outputs, smoke checks, alarms, and operational docs.
- Verify app authorization grant path for the operator account through `ahara-business-app-authorizations`.
- Exercise deploy flow without creating per-project VPC, ALB, RDS, API Gateway, or Cognito pool resources.
- Exit: `make ci` green; Terraform plans against Ahara modules and documented deploy/smoke paths are ready for operator execution.

## Decisions Needing Your Input

| Where | Decision you own |
| ---- | ---- |
| None | Core shape decisions are confirmed for this plan. |
