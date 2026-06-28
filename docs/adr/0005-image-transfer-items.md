# 0005 — Image Transfer Items

- Status: Accepted
- Date: 2026-06-23

## Context

Bookmarker supports phone-to-computer image transfer through the same capture
vault as URLs and text snippets. Image payloads need authenticated upload,
private storage, title/tags/notes, inbox state, watched state, search, and
desktop retrieval without turning the Android share flow into a multi-step form.

ADR-0004 keeps shared organization state on `items` and moves payload-specific
data into sibling tables.

## Decision

Image captures use `items.item_kind = image` with payload metadata in
`item_images`. The API creates a pending image item and an upload target, the
client uploads bytes to Linkdrop-owned storage, and a completion route marks the
image uploaded. Images share the same item title, tags, notes, inbox, watched
state, and detail UI as URL and text items.

## Alternatives considered

- **Treat images as URL attachments** — keeps the item model smaller, but image
  transfer from Android content URIs has no stable source URL.
- **Store image bytes in PostgreSQL** — simplifies retrieval semantics, but
  bloats the shared database and weakens object-level access controls.
- **Use public object URLs** — simplifies browser display, but exposes private
  personal-library images outside the authenticated API boundary.

## Consequences

The API owns upload-target creation, upload completion, and ownership checks for
image reads. Android streams shared image content to the issued upload target.
Web and desktop clients request short-lived presigned object-storage URLs from
the authenticated API, then load image bytes directly from object storage inside
the shared item detail surface. The API Lambda does not proxy original image
bytes.
