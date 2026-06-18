# 0004 — General Content Items

- Status: Accepted
- Date: 2026-06-17

## Context

Linkdrop started as URL capture, but the product now also needs short- and
medium-term text snippet capture from desktop clipboard workflows. URL drops and
text snippets need the same account, tags, notes, inbox, watched state, search,
and copy workflows without forcing URL-only metadata onto snippets.

## Decision

The durable root remains `items`. Payload-specific data lives in sibling tables:
`item_urls` for URL captures and `item_texts` for text snippets. `items.item_kind`
identifies the payload type. URL items keep URL normalization, enrichment, and
thumbnail snapshot processing. Text snippets keep their original text, optional
HTML/source metadata, and a per-user content hash for deduplication.

## Alternatives considered

- **Separate snippets table outside items** — simpler initially, but duplicates
  tags, notes, inbox, watched state, and list/filter behavior.
- **Stuff snippets into item notes** — avoids a migration, but confuses “why I
  saved this” notes with the actual captured content and weakens dedupe.
- **Single JSON payload column** — flexible, but makes constraints, search, and
  payload-specific indexes less explicit.

## Consequences

API and clients branch on `item_kind` while sharing the same organization
surface. URL-only processing must ignore text snippets. Text snippets report
`archive_status = not_applicable`, copy `text.plain_text`, and can coexist in
the same feed as URL captures.
