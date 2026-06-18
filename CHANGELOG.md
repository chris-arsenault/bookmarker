# Changelog

All notable user-visible changes are recorded here.

## Unreleased

- Added capture-time canonical URL storage for common tracking parameter
  stripping, `youtu.be` normalization, and best-effort `vt.tiktok.com` short-link
  resolution.
- Added per-user deduplication by canonical URL so repeated normalized captures
  return the existing item.
- Added `copy_url` to item responses so clients copy the canonical URL when
  available and fall back to the original URL when normalization fails.
- Added best-effort asynchronous metadata processing. Capture now queues the
  processing Lambda without waiting, processing writes `metadata_snapshots`
  with `archive_status`, and successful thumbnail archival stores a
  Linkdrop-owned `thumbnail_s3_key`.
- Failed enrichment now leaves the saved item visible with failed archive
  status instead of rejecting or deleting the drop.
- Added authenticated web library browsing with feed/detail views, view-only
  filters for platform, explicit tag, added date, `archive_status`, watched
  status, and title/notes search.
- Added authenticated API-mediated thumbnail reads for Linkdrop-owned snapshot
  objects.
- Added web item actions to open the source link and copy canonical `copy_url`.
- Added `inbox_status` with new captures defaulting to `unsorted` and explicit
  `organized` transitions after capture.
- Added item organization updates for notes editing, explicit tag replacement,
  watched/unwatched changes, and inbox status changes.
- Added tag rename and tag merge workflows that preserve item associations and
  usage counts for explicit tag corpus cleanup.
- Added Ahara deployment wiring for the API Lambda, processing Lambda, website,
  Cognito app client, auth-trigger registration, private snapshot bucket,
  runtime config, CloudWatch Lambda alarms, local deploy outputs, and
  post-deploy smoke checks.
