# Processing Lambda

Asynchronous Rust Lambda crate for Linkdrop metadata enrichment and snapshot
archival.

The Lambda accepts a typed event containing `item_id`, loads the saved item from
shared PostgreSQL, prefers the canonical URL when present, and falls back to the
original URL. Processing is idempotent: reruns update the same
`processing_jobs` and `metadata_snapshots` rows.

The pipeline marks `enrich_metadata` running, extracts provider or OpenGraph
metadata best effort, and stores title, author/channel, platform, optional
duration, `archive_status`, and `archive_error`. When metadata includes a
thumbnail URL, processing marks `snapshot_thumbnail` running, downloads the
thumbnail, writes it through the snapshot store boundary, and persists only the
Linkdrop-owned `thumbnail_s3_key` plus content type. Source thumbnail URLs are
not persisted as hotlinks.

Failures are terminal and visible rather than destructive: extraction or
snapshot errors upsert `archive_status = failed`, preserve the saved item, and
record a safe error string. Successful runs write `archive_status = succeeded`.

The deployed processing Lambda receives runtime IAM, private snapshot bucket
access, and CloudWatch Lambda alarms through Terraform. The API invokes this
Lambda asynchronously through `PROCESSING_FUNCTION_NAME`, and clients read
snapshots through the authenticated API thumbnail route.
