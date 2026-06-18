# 0001 — Ahara Platform Topology

- Status: Accepted
- Date: 2026-06-15

## Context

Linkdrop needs a central store, authenticated web access, an Android share client, async processing, and deployment through the Ahara platform.

Ahara provides shared ALB routing, Cognito authentication, PostgreSQL/RDS, Terraform state, and reusable Terraform modules for standard product apps.

## Decision

Linkdrop uses the standard Ahara topology: Rust Lambda backend behind the shared ALB, a React/Vite frontend on the `website` module, shared Cognito, shared PostgreSQL/RDS, and a native Android client that consumes the same authenticated API.

## Alternatives considered

- **Standalone service with its own database and auth** — gives isolation but duplicates platform infrastructure and access management.
- **Frontend-only storage** — reduces backend work but cannot provide cross-device sync, deduplication, enrichment, or archival guarantees.
- **Per-project API Gateway or RDS** — provides familiar AWS primitives but conflicts with Ahara platform constraints.

## Consequences

Linkdrop must be registered in `ahara-infra` for deployer permissions and database migrations. All HTTP backend traffic uses the shared ALB, and all user access uses the shared Cognito pool and app-authorization path.
