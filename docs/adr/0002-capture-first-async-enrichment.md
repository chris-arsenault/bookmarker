# 0002 — Capture-First Async Enrichment

- Status: Accepted
- Date: 2026-06-15

## Context

The Android share flow must save a link with one tap and no mandatory metadata fields. Metadata enrichment can be slow or fail because sources may be private, geo-blocked, deleted, or rate-limited.

## Decision

The capture API persists the drop immediately, then asynchronous processing normalizes URLs, deduplicates by canonical URL, enriches metadata, snapshots thumbnails, and records archive status.

## Alternatives considered

- **Blocking enrichment during capture** — improves immediate metadata quality but makes share-sheet capture fragile and slow.
- **Client-side enrichment** — keeps backend ingestion simple but duplicates parsing logic across clients and weakens archive consistency.
- **Manual-only metadata** — avoids source scraping risks but makes the visual library much less useful.

## Consequences

Every item has explicit processing and archive status. Failed enrichment is visible but does not delete or reject the saved link. API and UI flows must tolerate pending and failed metadata.
