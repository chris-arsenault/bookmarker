# 0003 — Project-Owned Snapshot Storage

- Status: Accepted
- Date: 2026-06-15

## Context

Linkdrop entries need to survive source deletion and source thumbnail changes. The feature spec requires thumbnail storage as a local copy rather than a hotlink.

## Decision

Linkdrop stores snapshot thumbnails in a project-owned private S3 bucket and persists snapshot metadata in PostgreSQL. Clients receive thumbnails through API-mediated access rather than direct source hotlinks.

## Alternatives considered

- **Source hotlinks** — simplest to implement but fails the archive requirement and leaks browsing behavior to source platforms.
- **Public S3 objects** — easy for browsers but makes archived thumbnails globally readable by object URL.
- **Shared `ahara-access` assets** — useful for grant-based sharing but heavier than Linkdrop's private personal-library snapshot need.

## Consequences

Terraform must create a snapshot bucket and grant Lambda read/write permissions. The API owns thumbnail access semantics, and processing records archive status for successful, failed, and pending snapshots.
